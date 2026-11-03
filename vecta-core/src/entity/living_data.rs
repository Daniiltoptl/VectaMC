//! Vanilla living-entity NBT load and save.
//!
//! The typed chunk record owns the base `Entity` fields, so this module covers
//! what `LivingEntity` adds on top: absorption, attributes, active effects,
//! health, hurt/death timers, fall-flying and sleeping state, combat
//! attribution, equipment and the impulse context. It mirrors
//! `LivingEntity.readAdditionalSaveData`/`addAdditionalSaveData` field by field.
//!
//! Brain and locator-bar icon persistence are not implemented in Vecta yet.

use std::str::FromStr;

use simdnbt::owned::{NbtList, NbtTag};
use vecta_registry::REGISTRY;
use vecta_registry::attribute::AttributeRef;
use vecta_utils::{BlockPos, Identifier, nbt::NbtNumeric as _};

use super::entity::{
    load_value_or, read_nbt_bool, read_nbt_dvec3, read_nbt_f32, read_nbt_int, read_nbt_int_stream,
    read_uuid,
};
use super::*;
use crate::entity::attribute::{AttributeModifier, AttributeModifierOperation};
use crate::entity::living_base::MobEffectInstance;

/// Saves the living-entity state vanilla persists.
pub(crate) fn save_living_data<E: LivingEntity + ?Sized>(entity: &E, nbt: &mut NbtCompound) {
    let living = entity.living_base();
    nbt.insert("Health", entity.get_health());
    nbt.insert("HurtTime", NbtTag::Short(living.hurt_time() as i16));
    nbt.insert("DeathTime", NbtTag::Short(living.death_time() as i16));
    nbt.insert("AbsorptionAmount", entity.get_absorption_amount());
    nbt.insert(
        "current_impulse_context_reset_grace_time",
        living.current_impulse_context_reset_grace_time(),
    );
    if let Some(impact) = living.current_impulse_impact_pos() {
        nbt.insert(
            "current_explosion_impact_pos",
            NbtList::Double(vec![impact.x, impact.y, impact.z]),
        );
    }
    nbt.insert("attributes", entity.attributes().lock().to_vanilla_nbt());

    let mut effects = living.active_mob_effects();
    effects.sort_by_key(|effect| effect.effect().try_id().unwrap_or(usize::MAX));
    if !effects.is_empty() {
        nbt.insert(
            "active_effects",
            NbtList::Compound(
                effects
                    .iter()
                    .map(MobEffectInstance::to_vanilla_nbt)
                    .collect(),
            ),
        );
    }

    nbt.insert("FallFlying", nbt_bool(entity.is_fall_flying()));
    if let Some(pos) = entity.sleeping_pos() {
        nbt.insert(
            "sleeping_pos",
            NbtTag::IntArray(vec![pos.x(), pos.y(), pos.z()]),
        );
    }
    if let Some(uuid) = entity.last_hurt_by_player_uuid() {
        nbt.insert(
            "last_hurt_by_player",
            NbtTag::IntArray(uuid.to_int_array().to_vec()),
        );
        nbt.insert(
            "last_hurt_by_player_memory_time",
            entity.last_hurt_by_player_memory_time(),
        );
    }
    if let Some(uuid) = entity.last_hurt_by_mob_uuid() {
        nbt.insert(
            "last_hurt_by_mob",
            NbtTag::IntArray(uuid.to_int_array().to_vec()),
        );
        nbt.insert(
            "ticks_since_last_hurt_by_mob",
            entity
                .tick_count()
                .wrapping_sub(entity.last_hurt_by_mob_timestamp()),
        );
    }

    entity.save_equipment(nbt);
}

/// Loads the living-entity state vanilla restores.
///
/// `reset` mirrors the two callers: a command or disk load resets every field
/// the payload omits (vanilla `Entity.load`), while spawn-item data merges into
/// the entity's current state.
pub(crate) fn load_living_data<E: LivingEntity + ?Sized>(
    entity: &E,
    nbt: &BorrowedNbtCompoundView<'_, '_>,
    reset: bool,
) {
    let living = entity.living_base();

    if let Some(absorption) = load_value_or(reset, read_nbt_f32(nbt, "AbsorptionAmount"), 0.0) {
        living.internal_set_absorption_amount(absorption);
    }

    load_attributes(entity, nbt);

    if reset || nbt.get("active_effects").is_some() {
        living.replace_active_mob_effects(read_effects(nbt).unwrap_or_default());
    }

    // Attributes load first, so the health clamp sees the loaded max health.
    if let Some(health) = load_value_or(reset, read_nbt_f32(nbt, "Health"), entity.get_max_health())
    {
        entity.set_health(health);
    }

    if let Some(hurt_time) = load_value_or(reset, read_nbt_int(nbt, "HurtTime"), 0) {
        living.set_hurt_time(i32::from(hurt_time as i16));
    }
    if let Some(death_time) = load_value_or(reset, read_nbt_int(nbt, "DeathTime"), 0) {
        living.set_death_time(i32::from(death_time as i16));
    }
    if let Some(fall_flying) = load_value_or(reset, read_nbt_bool(nbt, "FallFlying"), false) {
        entity.set_fall_flying(fall_flying);
    }

    load_sleeping_state(entity, nbt, reset);
    load_combat_references(entity, nbt, reset);

    // Vanilla clears every slot when the payload has no `equipment` compound.
    entity.load_equipment(*nbt, reset);

    let grace_time = load_value_or(
        reset,
        read_nbt_int(nbt, "current_impulse_context_reset_grace_time"),
        0,
    )
    .unwrap_or_else(|| living.current_impulse_context_reset_grace_time());
    let impact_pos = match read_nbt_dvec3(nbt, "current_explosion_impact_pos") {
        Some(impact) => Some(impact),
        None if reset => None,
        None => living.current_impulse_impact_pos(),
    };
    living.restore_impulse_context(grace_time, impact_pos);
}

/// Applies the packed `attributes` list.
///
/// Vanilla `AttributeMap.apply` only touches attributes the entity already has
/// and keeps transient modifiers that are not in the payload, so this never
/// rebuilds the map from scratch.
fn load_attributes<E: LivingEntity + ?Sized>(entity: &E, nbt: &BorrowedNbtCompoundView<'_, '_>) {
    let Some(packed_attributes) = nbt.list("attributes").and_then(|list| list.compounds()) else {
        return;
    };

    let mut attributes = entity.attributes().lock();
    for packed in packed_attributes {
        let Some(attribute) = read_attribute_key(&packed) else {
            continue;
        };

        // `ExtraCodecs.optionalAlwaysPresentFieldOf` defaults a missing base to 0.
        let base = packed
            .get("base")
            .and_then(|tag| tag.codec_f64())
            .unwrap_or(0.0);
        let modifiers = packed
            .list("modifiers")
            .and_then(|list| list.compounds())
            .into_iter()
            .flat_map(IntoIterator::into_iter)
            .filter_map(|modifier| read_attribute_modifier(&modifier))
            .collect();
        attributes.apply_packed(attribute, base, modifiers);
    }
}

fn read_attribute_key(nbt: &BorrowedNbtCompoundView<'_, '_>) -> Option<AttributeRef> {
    let key = Identifier::from_str(&nbt.string("id")?.to_string()).ok()?;
    REGISTRY.attributes.by_key(&key)
}

fn read_attribute_modifier(nbt: &BorrowedNbtCompoundView<'_, '_>) -> Option<AttributeModifier> {
    let id = Identifier::from_str(&nbt.string("id")?.to_string()).ok()?;
    let operation = AttributeModifierOperation::by_name(&nbt.string("operation")?.to_string())?;
    Some(AttributeModifier {
        id,
        amount: nbt.get("amount")?.codec_f64()?,
        operation,
    })
}

/// Retains the successfully decoded entries from a partial list result.
fn read_effects(nbt: &BorrowedNbtCompoundView<'_, '_>) -> Option<Vec<MobEffectInstance>> {
    let compounds = nbt.list("active_effects")?.compounds()?;
    Some(
        compounds
            .into_iter()
            .filter_map(|compound| MobEffectInstance::from_vanilla_nbt(&compound))
            .collect(),
    )
}

fn load_sleeping_state<E: LivingEntity + ?Sized>(
    entity: &E,
    nbt: &BorrowedNbtCompoundView<'_, '_>,
    reset: bool,
) {
    let sleeping_pos = read_nbt_int_stream(nbt, "sleeping_pos")
        .filter(|pos| pos.len() >= 3)
        .map(|pos| BlockPos::new(pos[0], pos[1], pos[2]));

    let Some(bed_position) = sleeping_pos else {
        if reset {
            entity.clear_sleeping_pos();
        }
        return;
    };

    entity.set_sleeping_pos(bed_position);
    entity.set_pose(EntityPose::Sleeping);
    // A freshly created entity is still on its first tick and keeps the saved
    // position, exactly like vanilla's `!this.firstTick` guard.
    if !entity.base().is_first_tick() && entity.set_pos_to_bed(bed_position).is_err() {
        entity.stop_sleeping();
    }
}

fn load_combat_references<E: LivingEntity + ?Sized>(
    entity: &E,
    nbt: &BorrowedNbtCompoundView<'_, '_>,
    reset: bool,
) {
    match read_uuid(nbt, "last_hurt_by_player") {
        Some(uuid) => entity.set_last_hurt_by_player(
            uuid,
            read_nbt_int(nbt, "last_hurt_by_player_memory_time").unwrap_or(0),
        ),
        None if reset => entity.living_base().clear_last_hurt_by_player(),
        None => {}
    }
    if reset || nbt.get("last_hurt_by_player_memory_time").is_some() {
        entity.living_base().set_last_hurt_by_player_memory_time(
            read_nbt_int(nbt, "last_hurt_by_player_memory_time").unwrap_or(0),
        );
    }

    // Vanilla stores the timestamp as "ticks since the hit", not an absolute one.
    let ticks_since_hit = read_nbt_int(nbt, "ticks_since_last_hurt_by_mob").unwrap_or(0);
    match read_uuid(nbt, "last_hurt_by_mob") {
        Some(uuid) => entity.living_base().set_last_hurt_by_mob_uuid(
            Some(uuid),
            entity.tick_count().wrapping_sub(ticks_since_hit),
        ),
        None if reset => entity
            .living_base()
            .set_last_hurt_by_mob_uuid(None, entity.tick_count().wrapping_sub(ticks_since_hit)),
        None => {}
    }
}
