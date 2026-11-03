//! Runtime entity attribute system.
//!
use core::iter;
use std::mem;

use simdnbt::owned::{NbtCompound, NbtList};
use vecta_protocol::packets::game::{AttributeModifierData, AttributeSnapshot};
pub use vecta_registry::attribute::AttributeModifierOperation;
use vecta_registry::attribute::AttributeRef;
use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::{REGISTRY, RegistryEntry, RegistryExt};
use vecta_utils::Identifier;

/// Growable bitmask for tracking dirty attribute IDs.
pub struct DirtySet {
    chunks: Vec<u64>,
}

impl Default for DirtySet {
    fn default() -> Self {
        Self::new()
    }
}

impl DirtySet {
    /// Creates an empty dirty set
    #[must_use]
    pub const fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Marks an attribute ID as dirty
    pub fn mark(&mut self, id: u16) {
        let chunk = id as usize / 64;
        let bit = id as usize % 64;
        if chunk >= self.chunks.len() {
            self.chunks.resize(chunk + 1, 0);
        }
        self.chunks[chunk] |= 1 << bit;
    }

    /// Returns `true` if no attributes are dirty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(|&c| c == 0)
    }

    /// Drains all marked IDs, clearing the set
    pub fn drain(&mut self) -> impl Iterator<Item = u16> + '_ {
        self.chunks.iter_mut().enumerate().flat_map(|(i, chunk)| {
            let mut bits = *chunk;
            *chunk = 0;
            iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let bit = bits.trailing_zeros() as u16;
                bits &= bits - 1;
                Some(i as u16 * 64 + bit)
            })
        })
    }
}

/// A modifier applied to an attribute instance
#[derive(Clone, Debug)]
pub struct AttributeModifier {
    /// Unique identifier (e.g. `minecraft:sprinting`)
    pub id: Identifier,
    /// The modifier value
    pub amount: f64,
    /// How the modifier is applied during calculation
    pub operation: AttributeModifierOperation,
}

/// Runtime state for a single attribute on an entity
pub struct AttributeInstance {
    attribute: AttributeRef,
    base_value: f64,
    modifiers: Vec<AttributeModifier>,
    modifiers_by_operation: [Vec<AttributeModifier>; 3],
    permanent: Vec<AttributeModifier>,
    cached_value: f64,
}

impl AttributeInstance {
    const fn new(attribute: AttributeRef, base_value: f64) -> Self {
        let cached = attribute.sanitize_value(base_value);
        Self {
            attribute,
            base_value,
            modifiers: Vec::new(),
            modifiers_by_operation: [Vec::new(), Vec::new(), Vec::new()],
            permanent: Vec::new(),
            cached_value: cached,
        }
    }

    /// Returns the attribute definition
    #[must_use]
    pub const fn attribute(&self) -> AttributeRef {
        self.attribute
    }

    /// Returns the base value before modifiers
    #[must_use]
    pub const fn base_value(&self) -> f64 {
        self.base_value
    }

    /// Sets the base value and recalculates. Returns `true` if changed
    #[expect(
        clippy::float_cmp,
        reason = "vanilla uses exact base value equality for dirty checks"
    )]
    pub fn set_base_value(&mut self, value: f64) -> bool {
        if self.base_value == value {
            return false;
        }
        self.base_value = value;
        self.recalculate();
        true
    }

    /// Returns the final calculated value (base + modifiers, clamped)
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.cached_value
    }

    /// Adds a modifier. Returns `false` if a modifier with this ID already exists.
    pub fn add_modifier(&mut self, modifier: AttributeModifier, persistent: bool) -> bool {
        if self.has_modifier(&modifier.id) {
            return false;
        }
        self.index_modifier(modifier, persistent);
        self.recalculate();
        true
    }

    /// Returns whether a modifier with the given ID exists.
    #[must_use]
    pub fn has_modifier(&self, id: &Identifier) -> bool {
        self.modifiers.iter().any(|modifier| modifier.id == *id)
    }

    const fn operation_index(operation: AttributeModifierOperation) -> usize {
        match operation {
            AttributeModifierOperation::AddValue => 0,
            AttributeModifierOperation::AddMultipliedBase => 1,
            AttributeModifierOperation::AddMultipliedTotal => 2,
        }
    }

    fn put_modifier(entries: &mut Vec<AttributeModifier>, modifier: AttributeModifier) {
        if let Some(existing) = entries.iter_mut().find(|entry| entry.id == modifier.id) {
            *existing = modifier;
        } else {
            entries.push(modifier);
        }
    }

    fn index_modifier(&mut self, modifier: AttributeModifier, persistent: bool) {
        let operation = Self::operation_index(modifier.operation);
        Self::put_modifier(
            &mut self.modifiers_by_operation[operation],
            modifier.clone(),
        );
        if persistent {
            Self::put_modifier(&mut self.permanent, modifier.clone());
        }
        Self::put_modifier(&mut self.modifiers, modifier);
    }

    /// Adds or replaces a modifier. Returns `true` if the value actually changed.
    #[expect(
        clippy::float_cmp,
        reason = "exact equality skips unchanged modifier updates"
    )]
    pub fn set_modifier(&mut self, modifier: AttributeModifier, persistent: bool) -> bool {
        if let Some(existing) = self.modifiers.iter().find(|entry| entry.id == modifier.id)
            && existing.amount == modifier.amount
            && existing.operation == modifier.operation
            && (!persistent
                || self.permanent.iter().any(|entry| {
                    entry.id == modifier.id
                        && entry.amount == modifier.amount
                        && entry.operation == modifier.operation
                }))
        {
            return false;
        }
        if persistent {
            self.remove_modifier(&modifier.id);
        }
        self.index_modifier(modifier, persistent);
        self.recalculate();
        true
    }

    /// Applies a packed instance without clearing unrelated operation entries.
    fn apply_packed(&mut self, base_value: f64, modifiers: Vec<AttributeModifier>) {
        self.base_value = base_value;
        // Vanilla maintains independent ID, operation and permanent indices.
        // Packed apply updates only the incoming operation; earlier operations
        // for the same ID continue participating in the value calculation.
        for modifier in modifiers {
            self.index_modifier(modifier, true);
        }
        self.recalculate();
    }

    /// Removes the modifier currently indexed by ID.
    pub fn remove_modifier(&mut self, id: &Identifier) -> bool {
        let Some(index) = self
            .modifiers
            .iter()
            .position(|modifier| modifier.id == *id)
        else {
            return false;
        };
        let removed = self.modifiers.remove(index);
        self.modifiers_by_operation[Self::operation_index(removed.operation)]
            .retain(|modifier| modifier.id != *id);
        self.permanent.retain(|modifier| modifier.id != *id);
        self.recalculate();
        true
    }

    /// Returns permanent modifiers for serialization.
    pub fn permanent_modifiers(&self) -> impl Iterator<Item = &AttributeModifier> {
        self.permanent.iter()
    }

    /// Removes transient modifiers from the ID and operation indices.
    fn remove_transient_modifiers(&mut self) -> bool {
        let mut changed = false;
        let mut index = 0;
        while index < self.modifiers.len() {
            if self
                .permanent
                .iter()
                .any(|modifier| modifier.id == self.modifiers[index].id)
            {
                index += 1;
            } else {
                let removed = self.modifiers.remove(index);
                self.modifiers_by_operation[Self::operation_index(removed.operation)]
                    .retain(|modifier| modifier.id != removed.id);
                changed = true;
            }
        }
        if changed {
            self.recalculate();
        }
        changed
    }

    /// Removes every ID-indexed modifier while preserving the base value.
    fn remove_modifiers(&mut self) -> bool {
        if self.modifiers.is_empty() {
            return false;
        }
        for modifier in mem::take(&mut self.modifiers) {
            self.modifiers_by_operation[Self::operation_index(modifier.operation)]
                .retain(|entry| entry.id != modifier.id);
            self.permanent.retain(|entry| entry.id != modifier.id);
        }
        self.recalculate();
        true
    }

    fn recalculate(&mut self) {
        let mut base = self.base_value;
        for modifier in &self.modifiers_by_operation[0] {
            base += modifier.amount;
        }
        let mut result = base;
        for modifier in &self.modifiers_by_operation[1] {
            result += base * modifier.amount;
        }
        for modifier in &self.modifiers_by_operation[2] {
            result *= 1.0 + modifier.amount;
        }
        self.cached_value = self.attribute.sanitize_value(result);
    }

    /// Builds a network snapshot for `CUpdateAttributes`
    fn to_snapshot(&self, attribute_id: i32) -> AttributeSnapshot {
        AttributeSnapshot {
            attribute_id,
            base_value: self.base_value,
            modifiers: self
                .modifiers
                .iter()
                .map(|m| AttributeModifierData {
                    id: m.id.clone(),
                    amount: m.amount,
                    operation: m.operation,
                })
                .collect(),
        }
    }
}

/// Per-entity container for attribute instances with dirty tracking
///
/// Indexed by attribute registry ID. Two dirty sets match vanilla's design:
/// - `to_update`: all dirty attributes, drained for server-side effects
/// - `to_sync`: syncable dirty attributes, drained for network packets
pub struct AttributeMap {
    instances: Vec<Option<AttributeInstance>>,
    to_update: DirtySet,
    to_sync: DirtySet,
}

impl AttributeMap {
    /// Creates an `AttributeMap` from an entity type's default attributes
    ///
    /// # Panics
    /// Panics if generated entity default attributes reference an attribute
    /// that is missing from the generated vanilla registry.
    // TODO: Add AttributeSupplier for lazy instantiation when mob entities are implemented
    #[must_use]
    pub fn new_for_entity(entity_type: EntityTypeRef) -> Self {
        let attr_count = REGISTRY.attributes.len();
        let mut instances = Vec::with_capacity(attr_count);
        instances.resize_with(attr_count, || None);

        for &(attr_name, base_value) in entity_type.default_attributes {
            let key = Identifier::vanilla_static(attr_name);
            let Some(id) = REGISTRY.attributes.id_from_key(&key) else {
                panic!(
                    "default attributes for entity type {} reference unregistered attribute {key}",
                    entity_type.key
                );
            };
            let Some(attr) = REGISTRY.attributes.by_id(id) else {
                panic!("attribute registry id {id} for default attribute {key} is not registered");
            };
            instances[id] = Some(AttributeInstance::new(attr, base_value));
        }

        Self {
            instances,
            to_update: DirtySet::new(),
            to_sync: DirtySet::new(),
        }
    }

    /// Returns `true` if the entity has this attribute registered
    #[must_use]
    pub fn has_attribute(&self, attribute: AttributeRef) -> bool {
        attribute
            .try_id()
            .and_then(|id| self.instances.get(id))
            .is_some_and(Option::is_some)
    }

    /// Gets the calculated value of an attribute
    #[must_use]
    pub fn get_value(&self, attribute: AttributeRef) -> Option<f64> {
        let id = attribute.try_id()?;
        self.instances
            .get(id)?
            .as_ref()
            .map(AttributeInstance::value)
    }

    /// Gets the calculated value of an attribute that must exist on this entity.
    ///
    /// # Panics
    ///
    /// Panics if the entity type was not constructed with the requested
    /// attribute. Vanilla's `AttributeSupplier.getValue` is a hard failure for
    /// missing attributes; using this keeps required living attributes from
    /// silently falling back to unrelated defaults.
    #[must_use]
    pub fn required_value(&self, attribute: AttributeRef) -> f64 {
        let Some(value) = self.get_value(attribute) else {
            panic!("required attribute {} is missing", attribute.key);
        };
        value
    }

    /// Gets the base value of an attribute
    #[must_use]
    pub fn get_base_value(&self, attribute: AttributeRef) -> Option<f64> {
        let id = attribute.try_id()?;
        self.instances
            .get(id)?
            .as_ref()
            .map(AttributeInstance::base_value)
    }

    /// Gets a reference to an attribute instance
    #[must_use]
    pub fn get_instance(&self, attribute: AttributeRef) -> Option<&AttributeInstance> {
        let id = attribute.try_id()?;
        self.instances.get(id)?.as_ref()
    }

    /// Serializes the vanilla `LivingEntity.attributes` list.
    #[must_use]
    pub(crate) fn to_vanilla_nbt(&self) -> NbtList {
        let attributes = self
            .instances
            .iter()
            .flatten()
            .map(|instance| {
                let mut attribute = NbtCompound::new();
                attribute.insert("id", instance.attribute().key.to_string());
                attribute.insert("base", instance.base_value());

                let modifiers = instance
                    .permanent_modifiers()
                    .map(|modifier| {
                        let mut packed = NbtCompound::new();
                        packed.insert("id", modifier.id.to_string());
                        packed.insert("amount", modifier.amount);
                        packed.insert("operation", modifier.operation.name());
                        packed
                    })
                    .collect::<Vec<_>>();
                if !modifiers.is_empty() {
                    attribute.insert("modifiers", NbtList::Compound(modifiers));
                }

                attribute
            })
            .collect();
        NbtList::Compound(attributes)
    }

    /// Returns whether an attribute has a modifier with the given ID.
    #[must_use]
    pub fn has_modifier(&self, attribute: AttributeRef, modifier_id: &Identifier) -> bool {
        self.get_instance(attribute)
            .is_some_and(|instance| instance.has_modifier(modifier_id))
    }

    /// Sets the base value of an attribute
    pub fn set_base_value(&mut self, attribute: AttributeRef, value: f64) {
        let Some(id) = attribute.try_id() else { return };
        let Some(Some(instance)) = self.instances.get_mut(id) else {
            return;
        };
        if instance.set_base_value(value) {
            self.mark_dirty(id, attribute);
        }
    }

    /// Assigns base values from attributes present in both maps.
    pub fn assign_base_values(&mut self, other: &Self) {
        let Self {
            instances,
            to_update,
            to_sync,
        } = self;

        for (id, other_instance) in other.instances.iter().enumerate() {
            let Some(other_instance) = other_instance else {
                continue;
            };
            let Some(Some(instance)) = instances.get_mut(id) else {
                continue;
            };
            if instance.set_base_value(other_instance.base_value()) {
                to_update.mark(id as u16);
                if instance.attribute.syncable {
                    to_sync.mark(id as u16);
                }
            }
        }
    }

    /// Adds permanent modifiers from attributes present in both maps.
    ///
    /// # Panics
    ///
    /// Panics if the destination already has a modifier with the same ID,
    /// matching vanilla's `AttributeInstance.addPermanentModifier` contract.
    pub fn assign_permanent_modifiers(&mut self, other: &Self) {
        let Self {
            instances,
            to_update,
            to_sync,
        } = self;

        for (id, other_instance) in other.instances.iter().enumerate() {
            let Some(other_instance) = other_instance else {
                continue;
            };
            let Some(Some(instance)) = instances.get_mut(id) else {
                continue;
            };

            for modifier in other_instance.permanent_modifiers() {
                assert!(
                    instance.add_modifier(modifier.clone(), true),
                    "modifier {} is already applied to attribute {}",
                    modifier.id,
                    instance.attribute.key
                );
                to_update.mark(id as u16);
                if instance.attribute.syncable {
                    to_sync.mark(id as u16);
                }
            }
        }
    }

    /// Adds a modifier to an attribute. Returns `false` if the modifier ID already exists
    pub fn add_modifier(
        &mut self,
        attribute: AttributeRef,
        modifier: AttributeModifier,
        persistent: bool,
    ) -> bool {
        let Some(id) = attribute.try_id() else {
            return false;
        };
        let Some(Some(instance)) = self.instances.get_mut(id) else {
            return false;
        };
        if instance.add_modifier(modifier, persistent) {
            self.mark_dirty(id, attribute);
            true
        } else {
            false
        }
    }

    /// Applies vanilla packed attribute state and marks the instance dirty.
    pub(crate) fn apply_packed(
        &mut self,
        attribute: AttributeRef,
        base_value: f64,
        modifiers: Vec<AttributeModifier>,
    ) {
        let Some(id) = attribute.try_id() else { return };
        let Some(Some(instance)) = self.instances.get_mut(id) else {
            return;
        };
        instance.apply_packed(base_value, modifiers);
        self.mark_dirty(id, attribute);
    }

    /// Adds or replaces a modifier on an attribute
    pub fn set_modifier(
        &mut self,
        attribute: AttributeRef,
        modifier: AttributeModifier,
        persistent: bool,
    ) {
        let Some(id) = attribute.try_id() else { return };
        let Some(Some(instance)) = self.instances.get_mut(id) else {
            return;
        };
        if instance.set_modifier(modifier, persistent) {
            self.mark_dirty(id, attribute);
        }
    }

    /// Removes a modifier from an attribute. Returns `true` if it existed
    pub fn remove_modifier(&mut self, attribute: AttributeRef, modifier_id: &Identifier) -> bool {
        let Some(id) = attribute.try_id() else {
            return false;
        };
        let Some(Some(instance)) = self.instances.get_mut(id) else {
            return false;
        };
        if instance.remove_modifier(modifier_id) {
            self.mark_dirty(id, attribute);
            true
        } else {
            false
        }
    }

    fn mark_dirty(&mut self, id: usize, attribute: AttributeRef) {
        let id = id as u16;
        self.to_update.mark(id);
        if attribute.syncable {
            self.to_sync.mark(id);
        }
    }

    /// Returns `true` if there are dirty attributes needing server-side effects
    #[must_use]
    pub fn has_dirty_updates(&self) -> bool {
        !self.to_update.is_empty()
    }

    /// Returns `true` if there are syncable dirty attributes needing network send
    #[must_use]
    pub fn has_dirty_sync(&self) -> bool {
        !self.to_sync.is_empty()
    }

    /// Drains `to_update` and returns the dirty attribute refs
    pub fn drain_dirty_updates(&mut self) -> Vec<AttributeRef> {
        self.to_update
            .drain()
            .filter_map(|id| {
                self.instances
                    .get(id as usize)?
                    .as_ref()
                    .map(|inst| inst.attribute)
            })
            .collect()
    }

    /// Drains `to_sync` and builds `AttributeSnapshot`s for `CUpdateAttributes`
    pub fn drain_dirty_sync(&mut self) -> Vec<AttributeSnapshot> {
        let ids: Vec<u16> = self.to_sync.drain().collect();
        let mut snapshots = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(Some(instance)) = self.instances.get(id as usize) {
                snapshots.push(instance.to_snapshot(i32::from(id)));
            }
        }
        snapshots
    }

    /// Returns snapshots for ALL syncable attributes (initial tracking sync)
    #[must_use]
    pub fn syncable_snapshots(&self) -> Vec<AttributeSnapshot> {
        let mut snapshots = Vec::new();
        for (id, slot) in self.instances.iter().enumerate() {
            if let Some(inst) = slot
                && inst.attribute.syncable
            {
                snapshots.push(inst.to_snapshot(id as i32));
            }
        }
        snapshots
    }

    /// Removes all transient modifiers (e.g. on respawn)
    pub fn remove_all_transient(&mut self) {
        let Self {
            instances,
            to_update,
            to_sync,
        } = self;
        for (id, slot) in instances.iter_mut().enumerate() {
            if let Some(inst) = slot
                && inst.remove_transient_modifiers()
            {
                to_update.mark(id as u16);
                if inst.attribute.syncable {
                    to_sync.mark(id as u16);
                }
            }
        }
    }

    /// Removes all permanent and transient modifiers while preserving base values
    pub fn remove_all_modifiers(&mut self) {
        let Self {
            instances,
            to_update,
            to_sync,
        } = self;
        for (id, slot) in instances.iter_mut().enumerate() {
            if let Some(inst) = slot
                && inst.remove_modifiers()
            {
                to_update.mark(id as u16);
                if inst.attribute.syncable {
                    to_sync.mark(id as u16);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
