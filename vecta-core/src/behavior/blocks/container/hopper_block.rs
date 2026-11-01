//! Hopper block behavior.
//!
//! A hopper is a five-slot container that moves one item per transfer tick: it
//! pushes into the container it faces and pulls from the container above it or
//! from item entities resting in its upper half. Its `enabled` state follows the
//! redstone signal around it, and a comparator reads its fullness.

use std::ptr;
use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::block_entity_type::BlockEntityTypeRef;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, EnumProperty,
};
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, axis::Axis, translations};

use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, BlockEntityCreation};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::HopperBlockEntity;
use crate::block_entity::{BLOCK_ENTITIES, BlockEntityTicker};
use crate::entity::ai::path::PathComputationType;
use crate::entity::{Entity, InsideBlockEffectCollector};
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::hopper_menu;
use crate::player::Player;
use crate::world::{LevelReader, SignalGetter as _, World};

const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING_HOPPER;
const ENABLED: &BoolProperty = &BlockStateProperties::ENABLED;

/// Vanilla hopper block behavior.
#[block_behavior]
pub struct HopperBlock {
    block: BlockRef,
}

impl HopperBlock {
    /// Creates a new hopper block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Vanilla `HopperBlock.checkPoweredState`: a hopper without a neighbour
    /// signal is enabled.
    fn check_powered_state(world: &Arc<World>, pos: BlockPos, state: BlockStateId) {
        let should_be_on = !world.has_neighbor_signal(pos);
        if should_be_on != state.get_value(ENABLED) {
            world.set_block(
                pos,
                state.set_value(ENABLED, should_be_on),
                UpdateFlags::UPDATE_CLIENTS,
            );
        }
    }
}

impl BlockBehavior for HopperBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // `FACING_HOPPER` has no up value, so a click on a top or bottom face
        // becomes a downward hopper.
        let facing = context.clicked_face().opposite();
        let facing = if facing.get_axis() == Axis::Y {
            Direction::Down
        } else {
            facing
        };
        Some(
            self.block
                .default_state()
                .set_value(FACING, facing)
                .set_value(ENABLED, true),
        )
    }

    fn on_place(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        old_state: BlockStateId,
        _moved_by_piston: bool,
    ) {
        // Vanilla `HopperBlock.onPlace` only re-checks the signal when the block
        // actually changed.
        if old_state.get_block() != state.get_block() {
            Self::check_powered_state(world, pos, state);
        }
    }

    fn handle_neighbor_changed(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _source_block: BlockRef,
        _moved_by_piston: bool,
    ) {
        Self::check_powered_state(world, pos, state);
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
        // Vanilla returns SUCCESS whether or not a menu opens.
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Success;
        };
        let Some(hopper) = block_entity.downcast_ref::<HopperBlockEntity>() else {
            return InteractionResult::Success;
        };

        let display_name = hopper
            .identity()
            .display_name(|| TextComponent::translated(translations::CONTAINER_HOPPER.msg()));
        // `RandomizableContainerBlockEntity.createMenu` refuses a locked hopper
        // after telling the player which item unlocks it.
        if !hopper.can_open(player) {
            player.close_container();
            if !player.is_spectator() {
                ContainerIdentity::send_locked_notification(world, player, pos, display_name);
            }
            player.award_custom_stat(&vanilla_custom_stats::INSPECT_HOPPER);
            return InteractionResult::Success;
        }

        // The same method rolls the pending loot table before the menu exists.
        hopper.unpack_loot_table(Some(player));

        let Some(container) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return InteractionResult::Success;
        };
        let inventory = Arc::clone(&player.inventory);
        player.open_menu(display_name, move |context| {
            hopper_menu(inventory, context.container_id, container)
        });

        player.award_custom_stat(&vanilla_custom_stats::INSPECT_HOPPER);
        InteractionResult::Success
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::HOPPER,
            level,
            pos,
            state,
        ))
    }

    fn get_block_entity_ticker(
        &self,
        _world: &Arc<World>,
        _state: BlockStateId,
        block_entity_type: BlockEntityTypeRef,
    ) -> Option<BlockEntityTicker> {
        // Vanilla `HopperBlock.getTicker` runs `pushItemsTick` on the server, and
        // only for the hopper block-entity type.
        ptr::eq(
            block_entity_type,
            &raw const vanilla_block_entity_types::HOPPER,
        )
        .then(|| {
            BlockEntityTicker::new(
                &vanilla_block_entity_types::HOPPER,
                HopperBlockEntity::push_items_tick,
            )
        })
    }

    fn entity_inside(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        entity: &dyn Entity,
        effect_collector: &mut InsideBlockEffectCollector,
        is_precise: bool,
    ) {
        // Vanilla `HopperBlock.entityInside` delegates to the block entity.
        if let Some(block_entity) = world.get_block_entity(pos)
            && let Some(hopper) = block_entity.downcast_ref::<HopperBlockEntity>()
        {
            hopper.entity_inside(world, pos, state, entity);
        }

        self.default_entity_inside(state, world, pos, entity, effect_collector, is_precise);
    }

    fn affect_neighbors_after_removal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        // Vanilla `Containers.updateNeighboursAfterDestroy`.
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
        // Vanilla `AbstractContainerMenu.getRedstoneSignalFromBlockEntity`.
        let Some(block_entity) = world.get_block_entity(pos) else {
            return 0;
        };
        if let Some(hopper) = block_entity.downcast_ref::<HopperBlockEntity>() {
            // Reading the slots unpacks a pending loot table first.
            hopper.unpack_loot_table(None);
        }
        let Some(container) = ContainerRef::from_block_entity(block_entity) else {
            return 0;
        };
        let guard = ContainerLockGuard::lock_all(&[&container]);
        guard
            .get(container.container_id())
            .map_or(0, calculate_redstone_signal_from_container)
    }

    fn is_pathfindable(
        &self,
        _state: BlockStateId,
        _computation_type: PathComputationType,
    ) -> bool {
        // Vanilla `HopperBlock.isPathfindable` is always false.
        false
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::slice;
    use std::sync::Arc;

    use glam::DVec3;
    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_registry::RegistryEntry as _;
    use vecta_registry::data_components::vanilla_components::CUSTOM_NAME;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::stat::vanilla_stat_types;
    use vecta_registry::{
        vanilla_blocks, vanilla_custom_stats, vanilla_entities, vanilla_items, vanilla_menu_types,
    };
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::behavior::context::{PlacementOrientation, PlacementSource};
    use crate::block_entity::entities::{
        FURNACE_SLOTS, HOPPER_SLOTS, LecternBlockEntity, MOVE_ITEM_SPEED, SLOT_FUEL, SLOT_INPUT,
        SLOT_RESULT,
    };
    use crate::block_entity::{BlockEntityLoadContext, SharedBlockEntity};
    use crate::bootstrap::init_globals;
    use crate::entity::entities::ItemEntity;
    use crate::entity::{SharedEntity, next_entity_id};
    use crate::player::{Player, ResetReason};
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, advance_test_game_time_to, fresh_test_world,
        insert_ready_full_chunk, opened_screens, recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    /// A hopper at [`TEST_POS`] in a loaded, block-ticking chunk.
    fn hopper_fixture(name: &'static str, facing: Direction) -> (TestWorld, BlockPos) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        let state = vanilla_blocks::HOPPER
            .default_state()
            .set_value(FACING, facing)
            .set_value(ENABLED, true);
        assert!(
            fixture
                .world
                .set_block(TEST_POS, state, UpdateFlags::UPDATE_ALL)
        );
        assert!(
            fixture.world.get_block_entity(TEST_POS).is_some(),
            "a placed hopper has a block entity"
        );
        (fixture, TEST_POS)
    }

    /// A hopper fixture whose viewer receives packets, so what the server sends
    /// can be inspected instead of assumed.
    fn listening_hopper_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        let (fixture, _pos) = hopper_fixture(name, Direction::Down);
        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "HopperListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the hopper's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the menu packet to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    fn place_chest(fixture: &TestWorld, pos: BlockPos) {
        assert!(fixture.world.set_block(
            pos,
            vanilla_blocks::CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
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

    fn hopper_at(fixture: &TestWorld, pos: BlockPos) -> SharedBlockEntity {
        fixture
            .world
            .get_block_entity(pos)
            .unwrap_or_else(|| panic!("{pos:?} should hold a hopper block entity"))
    }

    fn cooldown_at(fixture: &TestWorld, pos: BlockPos) -> i32 {
        hopper_at(fixture, pos)
            .downcast_ref::<HopperBlockEntity>()
            .expect("the hopper block creates a hopper block entity")
            .cooldown_time()
    }

    /// Runs the real block-entity ticker phase for `ticks` game ticks.
    fn tick_block_entities(fixture: &TestWorld, ticks: usize) {
        for _ in 0..ticks {
            advance_test_game_time_to(&fixture.world, fixture.world.game_time() + 1);
            fixture
                .world
                .block_entity_tickers()
                .tick(&fixture.world, true);
        }
    }

    fn hopper_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn spawn_item_entity(fixture: &TestWorld, position: DVec3, stack: ItemStack) -> SharedEntity {
        let entity: SharedEntity = Arc::new(ItemEntity::with_item(
            &vanilla_entities::ITEM,
            next_entity_id(),
            position,
            stack,
            Arc::downgrade(&fixture.world),
        ));
        fixture
            .world
            .try_add_entity(Arc::clone(&entity))
            .expect("the fixture chunk is loaded");
        entity
    }

    #[test]
    fn a_hopper_registers_the_push_items_ticker() {
        let (fixture, pos) = hopper_fixture("hopper_ticker", Direction::Down);
        let behavior = HopperBlock::new(&vanilla_blocks::HOPPER);
        let state = fixture.world.get_block_state(pos);

        assert_eq!(
            fixture.world.block_entity_tickers().registered_len(),
            1,
            "vanilla `HopperBlock.getTicker` registers exactly one ticker"
        );
        assert!(
            behavior
                .get_block_entity_ticker(&fixture.world, state, &vanilla_block_entity_types::HOPPER)
                .is_some()
        );
        assert!(
            behavior
                .get_block_entity_ticker(&fixture.world, state, &vanilla_block_entity_types::CHEST)
                .is_none(),
            "a hopper must not tick a foreign block entity"
        );
    }

    #[test]
    fn a_hopper_pushes_one_item_per_transfer_into_the_chest_below_it() {
        let (fixture, pos) = hopper_fixture("hopper_push", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 3),
        );

        tick_block_entities(&fixture, 1);

        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "vanilla moves exactly one item per transfer"
        );
        assert_eq!(item_at(&fixture, pos, 0).count(), 2);
        assert_eq!(
            cooldown_at(&fixture, pos),
            MOVE_ITEM_SPEED,
            "a successful transfer restarts the cooldown"
        );

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize - 1);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "the cooldown holds the next item back"
        );
        assert_eq!(cooldown_at(&fixture, pos), 1);

        tick_block_entities(&fixture, 1);
        assert_eq!(item_at(&fixture, below, 0).count(), 2);
        assert_eq!(item_at(&fixture, pos, 0).count(), 1);
    }

    #[test]
    fn a_hopper_pulls_one_item_per_transfer_from_the_container_above_it() {
        let (fixture, pos) = hopper_fixture("hopper_pull", Direction::Down);
        let above = pos.above();
        place_chest(&fixture, above);
        set_item_at(
            &fixture,
            above,
            0,
            ItemStack::with_count(&vanilla_items::DIAMOND, 3),
        );

        tick_block_entities(&fixture, 1);

        assert_eq!(item_at(&fixture, pos, 0).count(), 1);
        assert_eq!(item_at(&fixture, above, 0).count(), 2);

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert_eq!(item_at(&fixture, pos, 0).count(), 2);
        assert_eq!(item_at(&fixture, above, 0).count(), 1);
    }

    #[test]
    fn a_hopper_leaves_the_book_on_a_lectern_above_it() {
        // `LecternBlockEntity` is not a `Container`, so `getContainerAt` finds
        // nothing and the book stays put.
        let (fixture, pos) = hopper_fixture("hopper_lectern", Direction::Down);
        let above = pos.above();
        assert!(
            fixture.world.set_block(
                above,
                vanilla_blocks::LECTERN
                    .default_state()
                    .set_value(&BlockStateProperties::HAS_BOOK, true),
                UpdateFlags::UPDATE_ALL,
            )
        );
        let lectern = fixture
            .world
            .get_block_entity(above)
            .expect("a lectern has a block entity");
        let lectern = lectern
            .downcast_ref::<LecternBlockEntity>()
            .expect("the lectern block creates a lectern block entity");
        lectern.set_book(ItemStack::new(&vanilla_items::WRITTEN_BOOK));

        tick_block_entities(&fixture, 1);

        assert!(item_at(&fixture, pos, 0).is_empty());
        assert!(lectern.book().is(&vanilla_items::WRITTEN_BOOK));
    }

    #[test]
    fn a_hopper_rolls_the_loot_of_a_dropper_above_before_pulling() {
        // `RandomizableContainerBlockEntity.getItem` unpacks the loot table, so
        // the first slot read by the hopper already sees the rolled items.
        let (fixture, pos) = hopper_fixture("hopper_dropper_loot", Direction::Down);
        let above = pos.above();
        assert!(fixture.world.set_block(
            above,
            vanilla_blocks::DROPPER.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let dropper = fixture
            .world
            .get_block_entity(above)
            .expect("a dropper has a block entity");
        let mut nbt = NbtCompound::new();
        nbt.insert("LootTable", "minecraft:chests/simple_dungeon");
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("loot NBT decodes");
        dropper.load_additional(&borrowed);

        tick_block_entities(&fixture, 1);

        assert!(
            (0..HOPPER_SLOTS).any(|slot| !item_at(&fixture, pos, slot).is_empty()),
            "the rolled dungeon loot reaches the hopper"
        );
    }

    #[test]
    fn items_flow_from_the_chest_above_through_the_hopper_into_the_chest_below() {
        let (fixture, pos) = hopper_fixture("hopper_chain", Direction::Down);
        let above = pos.above();
        let below = pos.below();
        place_chest(&fixture, above);
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            above,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 5),
        );

        tick_block_entities(&fixture, 1);
        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "the hopper takes the first item from above"
        );
        assert_eq!(item_at(&fixture, above, 0).count(), 4);

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "the hopper pushes what it holds"
        );
        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "and pulls a replacement in the same transfer tick"
        );
        assert_eq!(item_at(&fixture, above, 0).count(), 3);
    }

    #[test]
    fn a_hopper_merges_into_an_existing_stack_and_moves_on_at_the_limit() {
        let (fixture, pos) = hopper_fixture("hopper_merge", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            below,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 63),
        );
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 2),
        );

        tick_block_entities(&fixture, 1);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            64,
            "the transfer tops the stack up to its limit"
        );
        assert_eq!(item_at(&fixture, pos, 0).count(), 1);

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            64,
            "a full stack is left alone"
        );
        assert_eq!(
            item_at(&fixture, below, 1).count(),
            1,
            "the next item goes to the next slot"
        );
        assert!(item_at(&fixture, pos, 0).is_empty());
    }

    #[test]
    fn a_full_destination_keeps_the_item_in_the_hopper() {
        let (fixture, pos) = hopper_fixture("hopper_blocked", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        for slot in 0..27 {
            set_item_at(
                &fixture,
                below,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize * 2);

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "a full chest accepts nothing, so the hopper keeps its item"
        );
        for slot in 0..27 {
            assert_eq!(item_at(&fixture, below, slot).count(), 64);
        }
    }

    #[test]
    fn a_powered_hopper_waits_until_the_signal_clears() {
        let (fixture, pos) = hopper_fixture("hopper_powered", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));

        assert!(
            fixture.world.get_block_state(pos).get_value(ENABLED),
            "a hopper without a neighbour signal is enabled"
        );
        assert!(fixture.world.set_block(
            pos.east(),
            vanilla_blocks::REDSTONE_BLOCK.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            !fixture.world.get_block_state(pos).get_value(ENABLED),
            "vanilla `HopperBlock.checkPoweredState` disables a hopper next to a signal"
        );

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize * 2);
        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "a disabled hopper does not transfer"
        );
        assert!(item_at(&fixture, below, 0).is_empty());

        assert!(fixture.world.set_block(
            pos.east(),
            vanilla_blocks::AIR.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            fixture.world.get_block_state(pos).get_value(ENABLED),
            "removing the signal re-enables the hopper"
        );

        tick_block_entities(&fixture, 1);
        assert_eq!(item_at(&fixture, below, 0).count(), 1);
        assert!(item_at(&fixture, pos, 0).is_empty());
    }

    #[test]
    fn a_hopper_picks_up_item_entities_resting_in_its_upper_half() {
        let (fixture, pos) = hopper_fixture("hopper_pickup", Direction::Down);

        // Below `Hopper.SUCK_AABB`, so the hopper must ignore it.
        let low = spawn_item_entity(
            &fixture,
            DVec3::new(4.5, 64.1, 4.5),
            ItemStack::with_count(&vanilla_items::STONE, 2),
        );
        let high = spawn_item_entity(
            &fixture,
            DVec3::new(4.5, 64.9, 4.5),
            ItemStack::with_count(&vanilla_items::DIAMOND, 3),
        );

        tick_block_entities(&fixture, 1);

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            3,
            "vanilla takes the whole item entity in one transfer"
        );
        assert!(ItemStack::is_same_item_same_components(
            &item_at(&fixture, pos, 0),
            &ItemStack::with_count(&vanilla_items::DIAMOND, 3),
        ));
        assert!(high.is_removed(), "a collected item entity is discarded");
        assert!(
            !low.is_removed(),
            "the lower entity is outside the suck area"
        );
        assert_eq!(
            low.as_ref()
                .downcast_ref::<ItemEntity>()
                .expect("the low entity is an item")
                .get_item()
                .count(),
            2
        );
    }

    #[test]
    fn entity_inside_moves_an_item_entity_immediately() {
        let (fixture, pos) = hopper_fixture("hopper_entity_inside", Direction::Down);
        let entity = spawn_item_entity(
            &fixture,
            DVec3::new(4.5, 64.9, 4.5),
            ItemStack::with_count(&vanilla_items::STONE, 2),
        );

        let behavior = HopperBlock::new(&vanilla_blocks::HOPPER);
        let mut collector = InsideBlockEffectCollector::new();
        behavior.entity_inside(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            entity.as_ref(),
            &mut collector,
            false,
        );

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            2,
            "vanilla `HopperBlockEntity.entityInside` moves the item without waiting for the tick"
        );
        assert!(entity.is_removed());
        assert_eq!(
            cooldown_at(&fixture, pos),
            MOVE_ITEM_SPEED,
            "the immediate move still restarts the transfer cooldown"
        );
    }

    #[test]
    fn opening_a_hopper_sends_the_hopper_menu_and_awards_the_inspect_stat() {
        let (fixture, player, packets) = listening_hopper_fixture("hopper_menu");
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);

        let result = HopperBlock::new(&vanilla_blocks::HOPPER).use_without_item(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
            &player,
            &hopper_hit(),
            &mut inventory,
        );

        assert_eq!(result, InteractionResult::Success);
        assert!(player.has_container_open());
        let screens = opened_screens(&packets);
        assert_eq!(screens.len(), 1, "opening a hopper sends one open-screen");
        assert_eq!(
            screens[0].1,
            vanilla_menu_types::HOPPER.id() as i32,
            "vanilla `HopperMenu` uses the five-slot hopper menu type"
        );
        assert_eq!(
            player.get_stat(vanilla_stat_types::CUSTOM.get(&vanilla_custom_stats::INSPECT_HOPPER)),
            1,
            "vanilla `HopperBlock.useWithoutItem` awards `inspect_hopper`"
        );
    }

    #[test]
    fn a_hopper_reports_its_fullness_to_a_comparator() {
        let (fixture, pos) = hopper_fixture("hopper_comparator", Direction::Down);
        let behavior = HopperBlock::new(&vanilla_blocks::HOPPER);
        let state = fixture.world.get_block_state(pos);
        let signal = |fixture: &TestWorld| {
            behavior.get_analog_output_signal(state, fixture.world.as_ref(), pos, Direction::Down)
        };

        assert!(behavior.has_analog_output_signal(state));
        assert_eq!(signal(&fixture), 0, "an empty hopper emits nothing");

        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 64),
        );
        assert_eq!(
            signal(&fixture),
            3,
            "one full slot out of five is a signal of three"
        );

        for slot in 1..HOPPER_SLOTS {
            set_item_at(
                &fixture,
                pos,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        assert_eq!(signal(&fixture), 15, "a full hopper emits a full signal");
    }

    #[test]
    fn a_restored_transfer_cooldown_delays_the_next_transfer() {
        let (fixture, pos) = hopper_fixture("hopper_cooldown_reload", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));

        tick_block_entities(&fixture, 1);
        assert_eq!(item_at(&fixture, below, 0).count(), 1);
        assert_eq!(cooldown_at(&fixture, pos), MOVE_ITEM_SPEED);

        // The chunk save path writes this tag and the load path reads it back,
        // so reloading must resume the cooldown instead of restarting it.
        let saved = hopper_at(&fixture, pos).save_with_full_metadata();
        assert_eq!(saved.int("TransferCooldown"), Some(MOVE_ITEM_SPEED));

        hopper_at(&fixture, pos)
            .load_with_owned_components(&saved, &BlockEntityLoadContext::detached())
            .expect("the saved hopper tag loads");
        assert_eq!(
            cooldown_at(&fixture, pos),
            MOVE_ITEM_SPEED,
            "the cooldown survives the save/load round trip"
        );

        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::STONE));
        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize - 1);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            1,
            "the restored cooldown still holds the item back"
        );

        tick_block_entities(&fixture, 1);
        assert_eq!(
            item_at(&fixture, below, 0).count(),
            2,
            "the transfer resumes exactly when the saved cooldown expires"
        );
    }

    #[test]
    fn a_hopper_faces_the_opposite_of_the_clicked_face() {
        let (fixture, _pos) = hopper_fixture("hopper_placement", Direction::Down);
        let behavior = HopperBlock::new(&vanilla_blocks::HOPPER);
        let mut stack = ItemStack::new(&vanilla_items::HOPPER);

        let mut placement_facing = |clicked_face: Direction| {
            let source = PlacementSource::direct(
                None,
                InteractionHand::MainHand,
                &mut stack,
                PlacementOrientation::Directional {
                    direction: clicked_face,
                },
                false,
            );
            let hit = BlockHitResult {
                location: DVec3::new(4.5, 64.5, 4.5),
                direction: clicked_face,
                block_pos: TEST_POS,
                miss: false,
                inside: false,
                world_border_hit: false,
            };
            let context = BlockPlaceContext::new(&fixture.world, source, &hit);
            behavior
                .get_state_for_placement(&context)
                .expect("a hopper always has a placement state")
                .get_value(FACING)
        };

        assert_eq!(
            placement_facing(Direction::East),
            Direction::West,
            "vanilla uses `clickedFace.getOpposite()`"
        );
        assert_eq!(
            placement_facing(Direction::Up),
            Direction::Down,
            "`FACING_HOPPER` has no up value, so a vertical click collapses to down"
        );
        assert_eq!(placement_facing(Direction::Down), Direction::Down);
    }

    #[test]
    fn a_hopper_is_never_pathfindable() {
        init_globals();
        let behavior = HopperBlock::new(&vanilla_blocks::HOPPER);
        let state = vanilla_blocks::HOPPER.default_state();

        for computation_type in [
            PathComputationType::Land,
            PathComputationType::Water,
            PathComputationType::Air,
        ] {
            assert!(
                !behavior.is_pathfindable(state, computation_type),
                "vanilla `HopperBlock.isPathfindable` is always false"
            );
        }
    }

    fn place_furnace(fixture: &TestWorld, pos: BlockPos) {
        assert!(fixture.world.set_block(
            pos,
            vanilla_blocks::FURNACE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    }

    /// A hopper above a furnace inserts through `getSlotsForFace(UP)`, which is
    /// the input slot only.
    #[test]
    fn a_hopper_above_a_furnace_feeds_only_the_input_slot() {
        let (fixture, pos) = hopper_fixture("hopper_furnace_top", Direction::Down);
        let furnace = pos.below();
        place_furnace(&fixture, furnace);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::IRON_ORE, 3),
        );

        tick_block_entities(&fixture, 1);

        assert_eq!(item_at(&fixture, furnace, SLOT_INPUT).count(), 1);
        assert!(
            item_at(&fixture, furnace, SLOT_FUEL).is_empty(),
            "the fuel slot is not reachable from above"
        );
        assert!(item_at(&fixture, furnace, SLOT_RESULT).is_empty());
        assert_eq!(item_at(&fixture, pos, 0).count(), 2);
    }

    /// A hopper beside a furnace inserts through `getSlotsForFace(side)`, which
    /// is the fuel slot only, and the fuel slot still applies `canPlaceItem`.
    #[test]
    fn a_hopper_beside_a_furnace_feeds_only_the_fuel_slot() {
        let (fixture, pos) = hopper_fixture("hopper_furnace_side", Direction::West);
        let furnace = pos.west();
        place_furnace(&fixture, furnace);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::COAL, 2),
        );

        tick_block_entities(&fixture, 1);

        assert_eq!(item_at(&fixture, furnace, SLOT_FUEL).count(), 1);
        assert!(
            item_at(&fixture, furnace, SLOT_INPUT).is_empty(),
            "the input slot is not reachable from the side"
        );

        // A non-fuel item stays in the hopper: `canPlaceItem` rejects it.
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::IRON_INGOT));
        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "the furnace refuses a non-fuel item in the fuel slot"
        );
        assert!(item_at(&fixture, furnace, SLOT_INPUT).is_empty());
    }

    /// A hopper below a furnace pulls through `getSlotsForFace(DOWN)`: the
    /// result slot always, the fuel slot only for `#furnace_fuel_bottom_takeable`.
    /// Vanilla always sucks through `Direction.DOWN`, whatever the hopper faces.
    #[test]
    fn a_hopper_below_a_furnace_takes_the_result_and_only_a_bucket_from_the_fuel_slot() {
        let (fixture, pos) = hopper_fixture("hopper_furnace_bottom", Direction::Down);
        let furnace = pos.above();
        place_furnace(&fixture, furnace);
        set_item_at(
            &fixture,
            furnace,
            SLOT_RESULT,
            ItemStack::new(&vanilla_items::IRON_INGOT),
        );

        tick_block_entities(&fixture, 1);

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "the result slot is reachable from below"
        );
        assert!(item_at(&fixture, furnace, SLOT_RESULT).is_empty());

        // Coal in the fuel slot is not extractable from below.
        set_item_at(
            &fixture,
            furnace,
            SLOT_FUEL,
            ItemStack::new(&vanilla_items::COAL),
        );
        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert!(
            item_at(&fixture, furnace, SLOT_FUEL).is(&vanilla_items::COAL),
            "vanilla refuses coal through the bottom face"
        );

        // An empty bucket is.
        set_item_at(
            &fixture,
            furnace,
            SLOT_FUEL,
            ItemStack::new(&vanilla_items::BUCKET),
        );
        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert!(item_at(&fixture, furnace, SLOT_FUEL).is_empty());
        assert!(
            (0..HOPPER_SLOTS).any(|slot| item_at(&fixture, pos, slot).is(&vanilla_items::BUCKET)),
            "`#furnace_fuel_bottom_takeable` lets the bucket out"
        );
    }

    /// A stack that differs in components must not merge, and a stack limit is
    /// never exceeded by a transfer.
    #[test]
    fn a_furnace_rejects_a_component_mismatch_and_never_overfills_a_slot() {
        let (fixture, pos) = hopper_fixture("hopper_furnace_mismatch", Direction::Down);
        let furnace = pos.below();
        place_furnace(&fixture, furnace);

        let plain = ItemStack::with_count(&vanilla_items::IRON_ORE, 1);
        let mut named = ItemStack::with_count(&vanilla_items::IRON_ORE, 1);
        named.set(CUSTOM_NAME, TextComponent::plain("Ore".to_owned()));
        assert!(!ItemStack::is_same_item_same_components(&plain, &named));

        set_item_at(&fixture, furnace, SLOT_INPUT, named);
        set_item_at(&fixture, pos, 0, plain);
        tick_block_entities(&fixture, 1);

        assert_eq!(
            item_at(&fixture, furnace, SLOT_INPUT).count(),
            1,
            "a component mismatch must not merge into the input stack"
        );
        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "the rejected item stays in the hopper"
        );

        // A full input slot takes nothing even though it is the right item.
        set_item_at(
            &fixture,
            furnace,
            SLOT_INPUT,
            ItemStack::with_count(&vanilla_items::IRON_ORE, 64),
        );
        set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::IRON_ORE));
        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize);
        assert_eq!(item_at(&fixture, furnace, SLOT_INPUT).count(), 64);
        assert_eq!(item_at(&fixture, pos, 0).count(), 1);
    }

    /// Both hoppers tick against the same furnace every tick; the item count is
    /// conserved and neither transfer path deadlocks on the furnace container.
    #[test]
    fn two_hoppers_move_items_through_a_furnace_without_duplication() {
        let (fixture, pos) = hopper_fixture("hopper_furnace_two_way", Direction::Down);
        let furnace = pos.below();
        place_furnace(&fixture, furnace);

        // The feeder above the furnace and the drain below it.
        let feeder = pos.above();
        assert!(
            fixture.world.set_block(
                feeder,
                vanilla_blocks::HOPPER
                    .default_state()
                    .set_value(FACING, Direction::Down)
                    .set_value(ENABLED, true),
                UpdateFlags::UPDATE_ALL,
            )
        );
        let drain = furnace.below();
        assert!(
            fixture.world.set_block(
                drain,
                vanilla_blocks::HOPPER
                    .default_state()
                    .set_value(FACING, Direction::Down)
                    .set_value(ENABLED, true),
                UpdateFlags::UPDATE_ALL,
            )
        );
        let sink = drain.below();
        place_chest(&fixture, sink);

        set_item_at(
            &fixture,
            feeder,
            0,
            ItemStack::with_count(&vanilla_items::IRON_ORE, 4),
        );
        set_item_at(
            &fixture,
            furnace,
            SLOT_RESULT,
            ItemStack::with_count(&vanilla_items::IRON_INGOT, 4),
        );

        let count_all = |fixture: &TestWorld| {
            let mut total = 0;
            for slot in 0..HOPPER_SLOTS {
                total += item_at(fixture, feeder, slot).count();
                total += item_at(fixture, pos, slot).count();
                total += item_at(fixture, drain, slot).count();
            }
            for slot in 0..FURNACE_SLOTS {
                total += item_at(fixture, furnace, slot).count();
            }
            for slot in 0..27 {
                total += item_at(fixture, sink, slot).count();
            }
            total
        };

        let initial = count_all(&fixture);
        assert_eq!(initial, 8);

        tick_block_entities(&fixture, MOVE_ITEM_SPEED as usize * 3);

        assert_eq!(
            count_all(&fixture),
            initial,
            "transfers must not duplicate or destroy items"
        );
        assert!(
            item_at(&fixture, sink, 0).count() > 0,
            "the drain hopper moves the furnace output into the chest below"
        );
        assert!(
            item_at(&fixture, furnace, SLOT_INPUT).count() > 0,
            "the feeder hopper fills the furnace input"
        );
    }
}
