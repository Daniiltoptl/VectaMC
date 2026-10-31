//! Custom name and lock shared by Vanilla `BaseContainerBlockEntity` containers.
//!
//! Rust has no block-entity inheritance, so the `CustomName` + `lock` pair that
//! Vanilla keeps on `BaseContainerBlockEntity` lives here and is embedded by
//! every container block entity that persists and enforces it.

use simdnbt::FromNbtTag as _;
use simdnbt::borrow::BaseNbtCompound as BorrowedNbtCompound;
use simdnbt::borrow::NbtCompound as NbtCompoundView;
use simdnbt::owned::NbtCompound;
use text_components::TextComponent;
use vecta_protocol::packets::game::SoundSource;
use vecta_registry::data_components::DataComponentMap;
use vecta_registry::data_components::vanilla_components::{CUSTOM_NAME, LOCK};
use vecta_registry::item_predicate::LockCode;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::sound_events;
use vecta_utils::types::InteractionHand;
use vecta_utils::{BlockPos, Identifier, locks::SyncMutex, translations};

use crate::entity::Entity as _;
use crate::player::Player;
use crate::world::World;

/// NBT keys the base container persists outside of item components.
pub const CUSTOM_NAME_TAG: &str = "CustomName";
/// NBT key holding the lock item predicate.
pub const LOCK_TAG: &str = "lock";

/// Custom name and lock of one container block entity.
#[derive(Default)]
pub struct ContainerIdentity {
    state: SyncMutex<ContainerIdentityState>,
}

#[derive(Default)]
struct ContainerIdentityState {
    name: Option<TextComponent>,
    lock: LockCode,
}

impl ContainerIdentity {
    /// Creates an unnamed, unlocked identity.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads `CustomName` and `lock` from a stored block-entity compound.
    pub fn load(&self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt: NbtCompoundView<'_, '_> = nbt.into();
        let name = nbt
            .get(CUSTOM_NAME_TAG)
            .and_then(|tag| TextComponent::from_nbt(&tag.to_owned()));
        let lock = nbt
            .get(LOCK_TAG)
            .and_then(LockCode::from_nbt_tag)
            .unwrap_or(LockCode::NO_LOCK);
        let mut state = self.state.lock();
        state.name = name;
        state.lock = lock;
    }

    /// Writes `CustomName` and `lock` into a stored block-entity compound.
    pub fn save(&self, nbt: &mut NbtCompound) {
        let state = self.state.lock();
        if let Some(name) = &state.name {
            nbt.insert(CUSTOM_NAME_TAG, name.to_codec_nbt());
        }
        if !state.lock.is_no_lock() {
            nbt.insert(LOCK_TAG, state.lock.to_nbt_tag_ref());
        }
    }

    /// Returns the custom name, if the container has one.
    #[must_use]
    pub fn custom_name(&self) -> Option<TextComponent> {
        self.state.lock().name.clone()
    }

    /// Returns the custom name, or `default` when unnamed.
    #[must_use]
    pub fn display_name(&self, default: impl FnOnce() -> TextComponent) -> TextComponent {
        self.state.lock().name.clone().unwrap_or_else(default)
    }

    /// Returns whether this container is locked at all.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        !self.state.lock().lock.is_no_lock()
    }

    /// Vanilla `BaseContainerBlockEntity.canOpen`: spectators always pass, and
    /// everyone else must hold a matching item in the main hand.
    #[must_use]
    pub fn can_open(&self, player: &Player) -> bool {
        if player.is_spectator() {
            return true;
        }
        let lock = self.state.lock().lock.clone();
        let inventory = player.inventory.lock();
        lock.unlocks_with(inventory.get_item_in_hand(InteractionHand::MainHand))
    }

    /// Vanilla `BaseContainerBlockEntity.sendChestLockedNotifications`.
    pub fn send_locked_notification(
        world: &World,
        player: &Player,
        pos: BlockPos,
        display_name: TextComponent,
    ) {
        player.send_overlay_message(&TextComponent::translated(
            translations::CONTAINER_IS_LOCKED.message([display_name]),
        ));
        world.play_sound_at(
            &sound_events::BLOCK_CHEST_LOCKED,
            SoundSource::Blocks,
            glam::DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
            1.0,
            1.0,
            None,
        );
    }

    /// Vanilla `BaseContainerBlockEntity.applyImplicitComponents`: the name and
    /// lock come from the item's components and are not stored twice.
    pub fn apply_implicit_components(&self, item: &ItemStack) -> Vec<Identifier> {
        let name = item.get(CUSTOM_NAME).cloned();
        let lock = item.get(LOCK).cloned().unwrap_or(LockCode::NO_LOCK);
        let mut state = self.state.lock();
        state.name = name;
        state.lock = lock;
        vec![CUSTOM_NAME.key().clone(), LOCK.key().clone()]
    }

    /// Vanilla `BaseContainerBlockEntity.collectImplicitComponents`.
    pub fn collect_implicit_components(&self, components: &mut DataComponentMap) {
        let state = self.state.lock();
        if let Some(name) = &state.name {
            components.set(CUSTOM_NAME, Some(name.clone()));
        }
        if !state.lock.is_no_lock() {
            components.set(LOCK, Some(state.lock.clone()));
        }
    }

    /// Vanilla `BaseContainerBlockEntity.removeComponentsFromTag`.
    pub fn remove_components_from_tag(nbt: &mut NbtCompound) {
        while nbt.remove(CUSTOM_NAME_TAG).is_some() {}
        while nbt.remove(LOCK_TAG).is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use vecta_registry::{init_vanilla_registry, vanilla_items};

    use super::*;

    fn round_trip(identity: &ContainerIdentity) -> ContainerIdentity {
        let mut nbt = NbtCompound::new();
        identity.save(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("identity NBT reads");
        let loaded = ContainerIdentity::new();
        loaded.load(&borrowed);
        loaded
    }

    #[test]
    fn custom_name_survives_a_disk_round_trip() {
        init_vanilla_registry();
        let identity = ContainerIdentity::new();
        identity.state.lock().name = Some(TextComponent::plain("Vault"));

        let loaded = round_trip(&identity);

        assert_eq!(
            loaded.display_name(|| TextComponent::plain("default")),
            TextComponent::plain("Vault"),
        );
    }

    #[test]
    fn an_unlocked_container_writes_no_lock_tag() {
        init_vanilla_registry();
        let identity = ContainerIdentity::new();

        let mut nbt = NbtCompound::new();
        identity.save(&mut nbt);

        assert!(nbt.get(LOCK_TAG).is_none());
        assert!(nbt.get(CUSTOM_NAME_TAG).is_none());
    }

    #[test]
    fn a_lock_predicate_survives_a_disk_round_trip() {
        init_vanilla_registry();
        let mut predicate = NbtCompound::new();
        predicate.insert("items", "minecraft:diamond");
        let mut nbt = NbtCompound::new();
        nbt.insert(LOCK_TAG, predicate);

        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("lock NBT reads");
        let identity = ContainerIdentity::new();
        identity.load(&borrowed);

        assert!(identity.is_locked());
        let loaded = round_trip(&identity);
        assert!(loaded.is_locked());
        let lock = loaded.state.lock().lock.clone();
        assert!(lock.unlocks_with(&ItemStack::new(&vanilla_items::DIAMOND)));
        assert!(!lock.unlocks_with(&ItemStack::new(&vanilla_items::STONE)));
    }
}
