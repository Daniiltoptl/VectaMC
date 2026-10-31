//! Dropper block behavior.
//!
//! A dropper is a nine-slot container that answers a redstone pulse by moving
//! one item out of a random non-empty slot: into the container it faces when
//! there is one, or as an item entity in front of it otherwise. Vanilla puts
//! this pulse logic on `DispenserBlock`, which `DropperBlock` extends; the only
//! dropper-specific part is that every item uses the plain drop behavior.

use std::slice;
use std::sync::{Arc, Weak};

use glam::DVec3;
use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::block_entity_type::BlockEntityTypeRef;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, EnumProperty,
};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, axis::Axis, translations};

use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, BlockEntityCreation};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::DropperBlockEntity;
use crate::block_entity::entities::push_into_container_at;
use crate::block_entity::{BLOCK_ENTITIES, BlockEntityTicker};
use crate::entity::Entity;
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::dispenser_menu;
use crate::player::Player;
use crate::world::{LevelReader, SignalGetter as _, World};

const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING;
const TRIGGERED: &BoolProperty = &BlockStateProperties::TRIGGERED;

/// Vanilla `DispenserBlock.TRIGGER_DURATION`: the scheduled delay between the
/// redstone edge and the dispense.
const TRIGGER_DURATION: i32 = 4;

/// Vanilla `DefaultDispenseItemBehavior.DEFAULT_ACCURACY`.
const DEFAULT_ACCURACY: f64 = 6.0;

/// Vanilla `DefaultDispenseItemBehavior` spawn offset along the facing axis.
const SPAWN_OFFSET: f64 = 0.7;

/// Vanilla dropper block behavior.
#[block_behavior]
pub struct DropperBlock {
    block: BlockRef,
}

impl DropperBlock {
    /// Creates a new dropper block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Vanilla `DropperBlock.dispenseFrom`: one item from a random non-empty
    /// slot, into the container in front or as an item entity.
    fn dispense_from(world: &Arc<World>, state: BlockStateId, pos: BlockPos) {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        let Some(dropper) = block_entity.downcast_ref::<DropperBlockEntity>() else {
            return;
        };
        let Some(slot) = dropper.get_random_slot(|bound| rand::random_range(0..bound)) else {
            // Vanilla plays the empty-dropper sound and changes nothing.
            world.level_event(level_events::SOUND_DISPENSER_FAIL, pos, 0, None);
            return;
        };
        let Some(container) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return;
        };
        let id = container.container_id();
        let stack = {
            let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
            guard
                .get(id)
                .map_or_else(ItemStack::empty, |locked| locked.get_item(slot).clone())
        };
        if stack.is_empty() {
            return;
        }

        let direction = state.get_value(FACING);
        let target_pos = pos.relative(direction);
        let mut one = stack.clone();
        one.set_count(1);

        // Vanilla takes the `into == null` branch, not "a block entity is
        // there": `HopperBlockEntity.getContainerAt` also returns null for a
        // block entity that is not a container, and such a dropper ejects.
        let moved = if let Some(remaining) =
            push_into_container_at(world, target_pos, &container, &one, direction.opposite())
        {
            remaining.is_empty()
        } else {
            Self::eject_one(world, pos, direction, one);
            true
        };

        if moved {
            let mut after = stack;
            after.shrink(1);
            let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
            guard.set_item(id, slot, after);
        }
    }

    /// Vanilla `DefaultDispenseItemBehavior`: one item entity in front of the
    /// block, with the vanilla spawn offset, velocity and level events.
    fn eject_one(world: &Arc<World>, pos: BlockPos, direction: Direction, item: ItemStack) {
        let (dx, _, dz) = direction.offset();
        let center = DVec3::new(
            f64::from(pos.x()) + 0.5,
            f64::from(pos.y()) + 0.5,
            f64::from(pos.z()) + 0.5,
        );
        let spawn = center + DVec3::from(direction.offset_vec()) * SPAWN_OFFSET;
        // Vanilla lowers the spawn point by 2/16 on the Y axis and 2.5/16 on the
        // horizontal axes so the item does not clip the block face.
        let y = spawn.y
            - if direction.get_axis() == Axis::Y {
                0.125
            } else {
                0.15625
            };

        let pow = rand::random::<f64>() * 0.1 + 0.2;
        let deviation = 0.017_227_5 * DEFAULT_ACCURACY;
        // `RandomSource.triangle(mean, deviation)`.
        let triangle =
            |mean: f64| mean + deviation * (rand::random::<f64>() - rand::random::<f64>());
        let velocity = DVec3::new(
            triangle(f64::from(dx) * pow),
            triangle(0.2),
            triangle(f64::from(dz) * pow),
        );

        world.spawn_item_with_velocity(DVec3::new(spawn.x, y, spawn.z), item, velocity);
        world.level_event(level_events::SOUND_DISPENSER_DISPENSE, pos, 0, None);
        world.level_event(
            level_events::PARTICLES_SHOOT_SMOKE,
            pos,
            direction.get_3d_data_value(),
            None,
        );
    }
}

impl BlockBehavior for DropperBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // Vanilla `DispenserBlock.getStateForPlacement`: the block faces away
        // from the player, so it points where the player is looking.
        Some(
            self.block
                .default_state()
                .set_value(FACING, context.get_nearest_looking_direction().opposite())
                .set_value(TRIGGERED, false),
        )
    }

    fn handle_neighbor_changed(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _source_block: BlockRef,
        _moved_by_piston: bool,
    ) {
        // Vanilla `DispenserBlock.neighborChanged`: the block above counts too,
        // a rising edge schedules the pulse, and a falling edge clears the flag.
        let should_trigger =
            world.has_neighbor_signal(pos) || world.has_neighbor_signal(pos.above());
        let is_triggered = state.get_value(TRIGGERED);
        if should_trigger && !is_triggered {
            world.schedule_block_tick_default(pos, self.block, TRIGGER_DURATION);
            world.set_block(
                pos,
                state.set_value(TRIGGERED, true),
                UpdateFlags::UPDATE_CLIENTS,
            );
        } else if !should_trigger && is_triggered {
            world.set_block(
                pos,
                state.set_value(TRIGGERED, false),
                UpdateFlags::UPDATE_CLIENTS,
            );
        }
    }

    fn tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        // Vanilla `DispenserBlock.tick` dispenses on the scheduled tick.
        Self::dispense_from(world, state, pos);
    }

    fn use_without_item(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
        _inv: &mut InventoryAccess,
    ) -> InteractionResult {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Success;
        };
        let Some(dropper) = block_entity.downcast_ref::<DropperBlockEntity>() else {
            return InteractionResult::Success;
        };

        let display_name = dropper
            .identity()
            .display_name(|| TextComponent::translated(translations::CONTAINER_DROPPER.msg()));
        if !dropper.can_open(player) {
            player.close_container();
            if !player.is_spectator() {
                ContainerIdentity::send_locked_notification(world, player, pos, display_name);
            }
            player.award_custom_stat(&vanilla_custom_stats::INSPECT_DROPPER);
            return InteractionResult::Success;
        }

        dropper.unpack_loot_table(Some(player));

        let Some(container) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return InteractionResult::Success;
        };
        let inventory = Arc::clone(&player.inventory);
        player.open_menu(display_name, move |context| {
            dispenser_menu(inventory, context.container_id, container)
        });

        player.award_custom_stat(&vanilla_custom_stats::INSPECT_DROPPER);
        InteractionResult::Success
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::DROPPER,
            level,
            pos,
            state,
        ))
    }

    fn get_block_entity_ticker(
        &self,
        _world: &Arc<World>,
        _state: BlockStateId,
        _block_entity_type: BlockEntityTypeRef,
    ) -> Option<BlockEntityTicker> {
        // Vanilla `DispenserBlock` has no block-entity ticker: the pulse runs on
        // the scheduled block tick.
        None
    }

    fn affect_neighbors_after_removal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        world.update_neighbor_for_output_signal(pos, self.block);
    }

    fn has_analog_output_signal(&self, _state: BlockStateId) -> bool {
        true
    }

    fn get_analog_output_signal(
        &self,
        _state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        _direction: Direction,
    ) -> i32 {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return 0;
        };
        if let Some(dropper) = block_entity.downcast_ref::<DropperBlockEntity>() {
            dropper.unpack_loot_table(None);
        }
        let Some(container) = ContainerRef::from_block_entity(block_entity) else {
            return 0;
        };
        let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        guard
            .get(container.container_id())
            .map_or(0, calculate_redstone_signal_from_container)
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;
    use std::slice;
    use std::sync::Arc;

    use glam::DVec3;
    use vecta_registry::RegistryEntry as _;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::level_events::{
        PARTICLES_SHOOT_SMOKE, SOUND_DISPENSER_DISPENSE, SOUND_DISPENSER_FAIL,
    };
    use vecta_registry::stat::vanilla_stat_types;
    use vecta_registry::{vanilla_blocks, vanilla_custom_stats, vanilla_items, vanilla_menu_types};
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::behavior::context::{PlacementOrientation, PlacementSource};
    use crate::block_entity::SharedBlockEntity;
    use crate::block_entity::entities::{CHEST_SLOTS, DROPPER_SLOTS};
    use crate::bootstrap::init_globals;
    use crate::entity::entities::ItemEntity;
    use crate::entity::{SharedEntity, next_entity_id};
    use crate::player::{Player, ResetReason};
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, dropped_items, fresh_test_world,
        insert_ready_full_chunk, level_events as sent_level_events, opened_screens,
        recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    /// Vanilla `DispenserBlock.TRIGGER_DURATION`, restated here so the test does
    /// not read the implementation's own constant.
    const VANILLA_TRIGGER_DURATION: i64 = 4;

    /// Vanilla `DefaultDispenseItemBehavior.DEFAULT_ACCURACY`.
    const VANILLA_DEFAULT_ACCURACY: f64 = 6.0;

    fn dropper_fixture(name: &'static str, facing: Direction) -> (TestWorld, BlockPos) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        let state = vanilla_blocks::DROPPER
            .default_state()
            .set_value(FACING, facing)
            .set_value(TRIGGERED, false);
        assert!(
            fixture
                .world
                .set_block(TEST_POS, state, UpdateFlags::UPDATE_ALL)
        );
        assert!(
            fixture.world.get_block_entity(TEST_POS).is_some(),
            "a placed dropper has a block entity"
        );
        (fixture, TEST_POS)
    }

    /// A dropper fixture whose viewer receives packets, so the level events and
    /// the open-screen can be inspected instead of assumed.
    fn listening_dropper_fixture(
        name: &'static str,
        facing: Direction,
    ) -> (TestWorld, BlockPos, Arc<Player>, SentPackets) {
        let (fixture, pos) = dropper_fixture(name, facing);
        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "DropperListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the dropper's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the level events to reach it"
        );
        packets.lock().clear();
        (fixture, pos, player, packets)
    }

    fn container_at(fixture: &TestWorld, pos: BlockPos) -> ContainerRef {
        let block_entity = fixture
            .world
            .get_block_entity(pos)
            .unwrap_or_else(|| panic!("{pos:?} should hold a block entity"));
        ContainerRef::from_block_entity(block_entity)
            .unwrap_or_else(|| panic!("the block entity at {pos:?} should be a container"))
    }

    fn item_at(fixture: &TestWorld, pos: BlockPos, slot: usize) -> ItemStack {
        let container = container_at(fixture, pos);
        let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        guard
            .get(container.container_id())
            .map_or_else(ItemStack::empty, |locked| locked.get_item(slot).clone())
    }

    fn set_item_at(fixture: &TestWorld, pos: BlockPos, slot: usize, stack: ItemStack) {
        let container = container_at(fixture, pos);
        let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        assert!(guard.set_item(container.container_id(), slot, stack));
    }

    fn total_items(fixture: &TestWorld, pos: BlockPos, slots: usize) -> i32 {
        (0..slots)
            .map(|slot| item_at(fixture, pos, slot).count())
            .sum()
    }

    fn place_chest(fixture: &TestWorld, pos: BlockPos) {
        assert!(fixture.world.set_block(
            pos,
            vanilla_blocks::CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    }

    fn dropper_at(fixture: &TestWorld, pos: BlockPos) -> SharedBlockEntity {
        fixture
            .world
            .get_block_entity(pos)
            .unwrap_or_else(|| panic!("{pos:?} should hold a dropper block entity"))
    }

    fn triggered(fixture: &TestWorld, pos: BlockPos) -> bool {
        fixture.world.get_block_state(pos).get_value(TRIGGERED)
    }

    /// Vanilla `DispenserBlock.tick` body, reached through the block behavior.
    fn pulse(behavior: &DropperBlock, fixture: &TestWorld, pos: BlockPos) {
        behavior.tick(fixture.world.get_block_state(pos), &fixture.world, pos);
    }

    fn neighbor_changed(behavior: &DropperBlock, fixture: &TestWorld, pos: BlockPos) {
        behavior.handle_neighbor_changed(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            &vanilla_blocks::REDSTONE_BLOCK,
            false,
        );
    }

    /// Item entities near the centre of `pos`, which is where a dropper spawns.
    fn ejected_items(fixture: &TestWorld, pos: BlockPos) -> Vec<SharedEntity> {
        dropped_items(
            &fixture.world,
            DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
        )
    }

    fn dropper_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    #[test]
    fn a_dropper_has_no_block_entity_ticker() {
        let (fixture, _pos) = dropper_fixture("dropper_ticker", Direction::Down);
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);
        let state = fixture.world.get_block_state(TEST_POS);

        assert_eq!(
            fixture.world.block_entity_tickers().registered_len(),
            0,
            "vanilla `DispenserBlock` has no ticker: the pulse runs on the scheduled block tick"
        );
        assert!(
            behavior
                .get_block_entity_ticker(
                    &fixture.world,
                    state,
                    &vanilla_block_entity_types::DROPPER
                )
                .is_none()
        );
        assert!(
            dropper_at(&fixture, TEST_POS)
                .downcast_ref::<DropperBlockEntity>()
                .is_some(),
            "the dropper block creates a dropper block entity"
        );
    }

    #[test]
    fn a_dropper_faces_away_from_the_player_look_direction() {
        let (fixture, _pos) = dropper_fixture("dropper_placement", Direction::Down);
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);
        let mut stack = ItemStack::new(&vanilla_items::DROPPER);

        let mut placement_facing = |rotation: f32, pitch: f32| {
            let source = PlacementSource::direct(
                None,
                InteractionHand::MainHand,
                &mut stack,
                PlacementOrientation::Player { rotation, pitch },
                false,
            );
            let hit = BlockHitResult {
                location: DVec3::new(4.5, 64.5, 4.5),
                direction: Direction::Up,
                block_pos: TEST_POS,
                miss: false,
                inside: false,
                world_border_hit: false,
            };
            let context = BlockPlaceContext::new(&fixture.world, source, &hit);
            let placed = behavior
                .get_state_for_placement(&context)
                .expect("a dropper always has a placement state");
            assert!(
                !placed.get_value(TRIGGERED),
                "vanilla places an untriggered dropper"
            );
            placed.get_value(FACING)
        };

        assert_eq!(
            placement_facing(0.0, 90.0),
            Direction::Up,
            "vanilla `getNearestLookingDirection().getOpposite()` makes a dropper looked at from above face up"
        );
        assert_eq!(
            placement_facing(180.0, 0.0),
            Direction::South,
            "a player looking north places a dropper that faces south"
        );
    }

    #[test]
    fn an_empty_dropper_answers_a_pulse_with_the_failure_sound() {
        let (fixture, pos, _player, packets) =
            listening_dropper_fixture("dropper_empty", Direction::Down);
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            sent_level_events(&packets),
            [(SOUND_DISPENSER_FAIL, pos, 0)],
            "vanilla `DropperBlock.dispenseFrom` sends level event 1001 for an empty dropper"
        );
        assert_eq!(total_items(&fixture, pos, DROPPER_SLOTS), 0);
        assert!(ejected_items(&fixture, pos).is_empty());
    }

    #[test]
    fn a_pulse_pushes_one_item_into_the_container_it_faces() {
        let (fixture, pos, _player, packets) =
            listening_dropper_fixture("dropper_push", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 3),
        );
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "vanilla `DropperBlock.dispenseFrom` moves one item into the container in front"
        );
        assert_eq!(item_at(&fixture, pos, 0).count(), 2);
        assert!(
            sent_level_events(&packets).is_empty(),
            "the container path does not run `DefaultDispenseItemBehavior`, so it plays no sound"
        );
        assert!(
            ejected_items(&fixture, pos).is_empty(),
            "a successful insertion spawns no item entity"
        );
    }

    #[test]
    fn a_full_neighbour_leaves_the_item_in_the_dropper() {
        let (fixture, pos, _player, packets) =
            listening_dropper_fixture("dropper_full", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        for slot in 0..CHEST_SLOTS {
            set_item_at(
                &fixture,
                below,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::DIAMOND));
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "a rejected item stays in its slot: vanilla keeps `itemStack.copy()`"
        );
        assert_eq!(total_items(&fixture, below, CHEST_SLOTS), 64 * 27);
        assert!(
            sent_level_events(&packets).is_empty(),
            "vanilla only plays the drop sound on the eject path"
        );
        assert!(ejected_items(&fixture, pos).is_empty());
    }

    #[test]
    fn a_pulse_ejects_one_item_when_nothing_is_in_front() {
        let (fixture, pos, _player, packets) =
            listening_dropper_fixture("dropper_eject", Direction::Down);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 2),
        );
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(item_at(&fixture, pos, 0).count(), 1, "one item leaves");
        let ejected = ejected_items(&fixture, pos);
        assert_eq!(ejected.len(), 1, "vanilla spawns exactly one item entity");
        let Some(item_entity) = ejected[0].downcast_ref::<ItemEntity>() else {
            panic!("an ejected item is an item entity");
        };
        assert!(item_entity.get_item().is(&vanilla_items::STONE));
        assert_eq!(item_entity.get_item().count(), 1);

        // `DispenserBlock.getDispensePosition` uses `source.center() + 0.7 *
        // direction`; a vertical facing then loses 2/16 instead of 2.5/16.
        let expected = DVec3::new(4.5, 64.5 - 0.7 - 0.125, 4.5);
        assert!(
            (ejected[0].position() - expected).length() < 1e-9,
            "ejected from {expected:?}, got {:?}",
            ejected[0].position()
        );
        let deviation = 0.017_227_5 * VANILLA_DEFAULT_ACCURACY;
        let velocity = ejected[0].velocity();
        assert!(
            velocity.x.abs() <= deviation && velocity.z.abs() <= deviation,
            "a vertical dropper only pushes along y, got {velocity:?}"
        );
        assert!(
            (velocity.y - 0.2).abs() <= deviation,
            "vanilla `random.triangle(0.2, 0.0172275 * 6)`, got {velocity:?}"
        );

        assert_eq!(
            sent_level_events(&packets),
            [
                (SOUND_DISPENSER_DISPENSE, pos, 0),
                (
                    PARTICLES_SHOOT_SMOKE,
                    pos,
                    Direction::Down.get_3d_data_value()
                ),
            ],
            "vanilla plays 1000 then the smoke animation 2000 with the facing data"
        );
    }

    #[test]
    fn a_horizontal_dropper_ejects_along_its_facing() {
        let (fixture, pos, _player, _packets) =
            listening_dropper_fixture("dropper_eject_side", Direction::South);
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        let ejected = ejected_items(&fixture, pos);
        assert_eq!(ejected.len(), 1);
        let expected = DVec3::new(4.5, 64.5 - 0.156_25, 4.5 + 0.7);
        assert!(
            (ejected[0].position() - expected).length() < 1e-9,
            "ejected from {expected:?}, got {:?}",
            ejected[0].position()
        );
        let velocity = ejected[0].velocity();
        assert!(
            velocity.z > 0.0,
            "a south-facing dropper throws the item south, got {velocity:?}"
        );
    }

    #[test]
    fn a_removed_destination_makes_the_dropper_eject() {
        let (fixture, pos, _player, _packets) =
            listening_dropper_fixture("dropper_removed", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        assert!(
            fixture.world.set_block(
                below,
                vanilla_blocks::AIR.default_state(),
                UpdateFlags::UPDATE_ALL
            ),
            "the destination can be removed"
        );
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert!(
            item_at(&fixture, pos, 0).is_empty(),
            "with no container in front the item leaves the dropper"
        );
        assert_eq!(
            ejected_items(&fixture, pos).len(),
            1,
            "vanilla falls back to `DefaultDispenseItemBehavior`"
        );
    }

    #[test]
    fn a_non_container_block_entity_in_front_does_not_swallow_the_pulse() {
        let (fixture, pos, _player, _packets) =
            listening_dropper_fixture("dropper_sign", Direction::Down);
        let below = pos.below();
        assert!(
            fixture.world.set_block(
                below,
                vanilla_blocks::OAK_SIGN.default_state(),
                UpdateFlags::UPDATE_ALL
            ),
            "the sign can be placed"
        );
        let sign = fixture
            .world
            .get_block_entity(below)
            .expect("a placed sign has a block entity");
        assert!(
            ContainerRef::from_block_entity(sign).is_none(),
            "a sign is a block entity but not a container"
        );
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            ejected_items(&fixture, pos).len(),
            1,
            "vanilla tests `HopperBlockEntity.getContainerAt(...) == null`, not `getBlockEntity != null`"
        );
        assert!(item_at(&fixture, pos, 0).is_empty());
    }

    #[test]
    fn a_long_powered_signal_dispenses_once_per_rising_edge() {
        let (fixture, pos) = dropper_fixture("dropper_signal", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 3),
        );
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);
        let now = fixture.world.game_time();

        // Powering a neighbour is what vanilla turns into `neighborChanged`.
        assert!(fixture.world.set_block(
            pos.east(),
            vanilla_blocks::REDSTONE_BLOCK.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            triggered(&fixture, pos),
            "a rising edge latches `TRIGGERED`"
        );
        assert!(
            fixture
                .world
                .has_scheduled_block_tick(pos, &vanilla_blocks::DROPPER)
        );

        let early = fixture
            .world
            .begin_scheduled_tick_phase(now + VANILLA_TRIGGER_DURATION - 1, 64);
        assert!(
            early.ticks.is_empty(),
            "vanilla `scheduleTick(pos, this, 4)` does not fire early"
        );
        let due = fixture
            .world
            .begin_scheduled_tick_phase(now + VANILLA_TRIGGER_DURATION, 64);
        assert_eq!(due.ticks.len(), 1);
        assert_eq!(due.ticks[0].pos, pos);
        assert_eq!(due.ticks[0].trigger_tick, now + VANILLA_TRIGGER_DURATION);
        assert!(ptr::eq(
            due.ticks[0].tick_type,
            &raw const vanilla_blocks::DROPPER
        ));

        // A still-powered dropper keeps its latch and never schedules again.
        neighbor_changed(&behavior, &fixture, pos);
        assert!(triggered(&fixture, pos));
        assert!(
            !fixture
                .world
                .has_scheduled_block_tick(pos, &vanilla_blocks::DROPPER),
            "vanilla only schedules on the rising edge"
        );

        pulse(&behavior, &fixture, pos);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "one pulse moves one item"
        );

        // The falling edge clears the latch without dispensing.
        assert!(fixture.world.set_block(
            pos.east(),
            vanilla_blocks::AIR.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            !triggered(&fixture, pos),
            "the falling edge clears `TRIGGERED`"
        );
        assert!(
            !fixture
                .world
                .has_scheduled_block_tick(pos, &vanilla_blocks::DROPPER),
            "the falling edge does not schedule a dispense"
        );
        assert_eq!(item_at(&fixture, below, 0).count(), 1);
        assert_eq!(item_at(&fixture, pos, 0).count(), 2);
    }

    #[test]
    fn two_rising_edges_dispense_two_items() {
        let (fixture, pos) = dropper_fixture("dropper_two_pulses", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 3),
        );
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        let power = |on: bool| {
            let state = if on {
                vanilla_blocks::REDSTONE_BLOCK.default_state()
            } else {
                vanilla_blocks::AIR.default_state()
            };
            assert!(
                fixture
                    .world
                    .set_block(pos.east(), state, UpdateFlags::UPDATE_ALL)
            );
        };
        let run_scheduled_pulse = || {
            let now = fixture.world.game_time();
            let batch = fixture
                .world
                .begin_scheduled_tick_phase(now + VANILLA_TRIGGER_DURATION, 64);
            assert_eq!(batch.ticks.len(), 1, "each edge schedules exactly one tick");
            pulse(&behavior, &fixture, pos);
        };

        power(true);
        run_scheduled_pulse();
        assert_eq!(item_at(&fixture, below, 0).count(), 1);

        power(false);
        power(true);
        run_scheduled_pulse();

        assert_eq!(
            item_at(&fixture, below, 0).count(),
            2,
            "two pulses move two items"
        );
        assert_eq!(item_at(&fixture, pos, 0).count(), 1);
    }

    #[test]
    fn one_pulse_moves_exactly_one_item_from_nine_full_slots() {
        let (fixture, pos) = dropper_fixture("dropper_one_item", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        for slot in 0..DROPPER_SLOTS {
            set_item_at(
                &fixture,
                pos,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            total_items(&fixture, pos, DROPPER_SLOTS),
            9 * 64 - 1,
            "vanilla takes one item from one random slot, not every stack"
        );
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "the neighbour receives a single item"
        );
    }

    #[test]
    fn opening_a_dropper_sends_the_generic_3x3_menu_and_awards_the_inspect_stat() {
        let (fixture, pos, player, packets) =
            listening_dropper_fixture("dropper_menu", Direction::Down);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);

        let result = DropperBlock::new(&vanilla_blocks::DROPPER).use_without_item(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            &player,
            &dropper_hit(),
            &mut inventory,
        );

        assert_eq!(result, InteractionResult::Success);
        assert!(player.has_container_open());
        let screens = opened_screens(&packets);
        assert_eq!(screens.len(), 1, "opening a dropper sends one open-screen");
        assert_eq!(
            screens[0].1,
            vanilla_menu_types::GENERIC_3X3.id() as i32,
            "vanilla `DispenserMenu` uses the generic 3x3 menu type for droppers too"
        );
        assert_eq!(
            player.get_stat(vanilla_stat_types::CUSTOM.get(&vanilla_custom_stats::INSPECT_DROPPER)),
            1,
            "vanilla awards `inspect_dropper`, not `inspect_dispenser`"
        );
    }

    #[test]
    fn a_dropper_reports_its_fullness_to_a_comparator() {
        let (fixture, pos) = dropper_fixture("dropper_comparator", Direction::Down);
        let behavior = DropperBlock::new(&vanilla_blocks::DROPPER);
        let state = fixture.world.get_block_state(pos);
        let signal = |fixture: &TestWorld| {
            behavior.get_analog_output_signal(state, fixture.world.as_ref(), pos, Direction::Down)
        };

        assert!(behavior.has_analog_output_signal(state));
        assert_eq!(signal(&fixture), 0, "an empty dropper emits nothing");

        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 64),
        );
        assert_eq!(
            signal(&fixture),
            2,
            "one full slot out of nine is `floor(14 / 9) + 1`"
        );

        for slot in 1..DROPPER_SLOTS {
            set_item_at(
                &fixture,
                pos,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        assert_eq!(signal(&fixture), 15, "a full dropper emits a full signal");
    }
}
