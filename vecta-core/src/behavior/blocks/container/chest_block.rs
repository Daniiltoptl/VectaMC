//! Chest block behavior implementation.
//!
//! A chest opens a 27-slot container menu, tracks its viewers so clients can
//! animate the lid, and exposes its fullness to comparators.

use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, ChestType, Direction, EnumProperty,
};
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, translations};

use crate::behavior::BlockEntityCreation;
use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, schedule_water_tick_if_waterlogged};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::BLOCK_ENTITIES;
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::ChestBlockEntity;
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::{ChestKind, chest_with_kind};
use crate::player::Player;
use crate::world::{LevelReader, ScheduledTickAccess, World, is_redstone_conductor};

/// Number of rows a chest menu shows.
const CHEST_ROWS: usize = 3;

const FACING: &EnumProperty<Direction> = &BlockStateProperties::HORIZONTAL_FACING;
const WATERLOGGED: &BoolProperty = &BlockStateProperties::WATERLOGGED;

/// Behavior for chest blocks.
#[block_behavior]
pub struct ChestBlock {
    block: BlockRef,
}

impl ChestBlock {
    /// Creates a new chest block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// `ChestBlock.isChestBlockedAt`: a redstone conductor above the chest
    /// keeps it shut. The sitting-cat half needs cats, which Vecta lacks.
    fn is_blocked(world: &World, pos: BlockPos) -> bool {
        let above = pos.above();
        is_redstone_conductor(world, world.get_block_state(above), above)
    }
}

impl BlockBehavior for ChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // A single chest faces the player; joining two halves is a separate
        // placement step that also rewrites the neighbour's chest type.
        Some(
            self.block
                .default_state()
                .set_value(FACING, context.horizontal_direction().opposite())
                .set_value(&BlockStateProperties::CHEST_TYPE, ChestType::Single)
                .set_value(WATERLOGGED, context.is_water_source()),
        )
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        world: &dyn ScheduledTickAccess,
        pos: BlockPos,
        _direction: Direction,
        _neighbor_pos: BlockPos,
        _neighbor_state: BlockStateId,
    ) -> BlockStateId {
        // Neighbour chest joining is W04-022.
        schedule_water_tick_if_waterlogged(state, world, pos);
        state
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
        if Self::is_blocked(world, pos) {
            return InteractionResult::Success;
        }
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Success;
        };
        let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() else {
            return InteractionResult::Success;
        };
        let display_name = chest
            .identity()
            .display_name(|| TextComponent::translated(translations::CONTAINER_CHEST.msg()));
        // `BaseContainerBlockEntity.createMenu` refuses to build a menu for a
        // locked container after telling the player which item unlocks it.
        if !chest.identity().can_open(player) {
            player.close_container();
            ContainerIdentity::send_locked_notification(world, player, pos, display_name);
            player.award_custom_stat(&vanilla_custom_stats::OPEN_CHEST);
            return InteractionResult::Success;
        }
        let Some(container_ref) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return InteractionResult::Success;
        };

        let inventory = Arc::clone(&player.inventory);
        player.open_menu(display_name, move |context| {
            // `ChestMenu` starts the opener in its constructor, after the
            // previous menu has been closed.
            if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
                chest.start_open(context.player);
            }
            let container = container_ref.clone();
            chest_with_kind(
                inventory,
                context.container_id,
                container.clone(),
                CHEST_ROWS,
                ChestKind::new(container).with_close_hook(move |player| {
                    if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
                        chest.stop_open(player);
                    }
                }),
            )
        });

        player.award_custom_stat(&vanilla_custom_stats::OPEN_CHEST);
        // TODO: Anger nearby piglins (PiglinAi.angerNearbyPiglins) once piglin AI exists.
        InteractionResult::Success
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::CHEST,
            level,
            pos,
            state,
        ))
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
        let Some(container_ref) = world
            .get_block_entity(pos)
            .and_then(ContainerRef::from_block_entity)
        else {
            return 0;
        };
        let guard = ContainerLockGuard::lock_all(&[&container_ref]);
        guard
            .get(container_ref.container_id())
            .map_or(0, |container| {
                calculate_redstone_signal_from_container(container)
            })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::{Arc, Weak};

    use vecta_registry::data_components::DataComponentMap;
    use vecta_registry::data_components::vanilla_components::CONTAINER;

    use glam::DVec3;
    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_registry::{item_stack::ItemStack, vanilla_blocks, vanilla_items};
    use vecta_utils::types::{GameType, InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::block_entity::BlockEntity;
    use crate::bootstrap::init_globals;
    use crate::entity::Entity as _;
    use crate::test_support::{
        TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    fn chest_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "ChestUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn chest_hit() -> BlockHitResult {
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
    fn opening_a_chest_counts_the_viewer_and_closing_releases_it() {
        let (fixture, player) = chest_fixture("chest_open_close");
        let behavior = ChestBlock::new(&vanilla_blocks::CHEST);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity");
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");

        let result = behavior.use_without_item(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
            &player,
            &chest_hit(),
            &mut inventory,
        );

        assert_eq!(result, InteractionResult::Success);
        assert_eq!(chest.open_count(), 1);

        player.close_container();
        assert_eq!(chest.open_count(), 0);
    }

    fn use_chest(fixture: &TestWorld, player: &Arc<Player>) -> InteractionResult {
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        ChestBlock::new(&vanilla_blocks::CHEST).use_without_item(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
            player,
            &chest_hit(),
            &mut inventory,
        )
    }

    fn open_count(fixture: &TestWorld) -> i32 {
        fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity")
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity")
            .open_count()
    }

    #[test]
    fn a_conductor_above_keeps_the_chest_shut() {
        let (fixture, player) = chest_fixture("chest_blocked");
        assert!(fixture.world.set_block(
            TEST_POS.above(),
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        assert_eq!(use_chest(&fixture, &player), InteractionResult::Success);
        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn a_spectator_does_not_count_as_a_viewer() {
        let (fixture, player) = chest_fixture("chest_spectator");
        player.restore_game_modes(GameType::Spectator, None);

        assert_eq!(use_chest(&fixture, &player), InteractionResult::Success);
        assert_eq!(open_count(&fixture), 0);
        player.close_container();
        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn reopening_the_same_chest_closes_the_previous_view_first() {
        let (fixture, player) = chest_fixture("chest_reopen");

        use_chest(&fixture, &player);
        use_chest(&fixture, &player);
        assert_eq!(open_count(&fixture), 1);

        player.close_container();
        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn chest_fullness_drives_the_comparator_signal() {
        let (fixture, _player) = chest_fixture("chest_comparator");
        let behavior = ChestBlock::new(&vanilla_blocks::CHEST);
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity");
        let container_ref = block_entity
            .container_ref()
            .expect("a chest exposes its container");
        let state = fixture.world.get_block_state(TEST_POS);

        assert_eq!(
            behavior.get_analog_output_signal(state, &fixture.world, TEST_POS, Direction::South),
            0
        );

        {
            let mut guard = ContainerLockGuard::lock_all(&[&container_ref]);
            guard
                .get_mut(container_ref.container_id())
                .expect("the chest container is locked")
                .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 64));
        }

        assert!(
            behavior.get_analog_output_signal(state, &fixture.world, TEST_POS, Direction::South)
                > 0
        );
    }

    #[test]
    fn chest_contents_survive_an_nbt_round_trip() {
        let (fixture, _player) = chest_fixture("chest_nbt_round_trip");
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity");
        let container_ref = block_entity
            .container_ref()
            .expect("a chest exposes its container");
        {
            let mut guard = ContainerLockGuard::lock_all(&[&container_ref]);
            let container = guard
                .get_mut(container_ref.container_id())
                .expect("the chest container is locked");
            container.set_item(0, ItemStack::new(&vanilla_items::DIAMOND));
            container.set_item(26, ItemStack::with_count(&vanilla_items::STONE, 3));
        }

        let mut nbt = NbtCompound::new();
        block_entity.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("chest NBT decodes");

        let loaded =
            ChestBlockEntity::new(Weak::new(), TEST_POS, vanilla_blocks::CHEST.default_state());
        loaded.load_additional(&borrowed);

        let container = loaded
            .container_ref()
            .expect("a chest exposes its container");
        let guard = ContainerLockGuard::lock_all(&[&container]);
        let container = guard
            .get(container.container_id())
            .expect("the loaded chest container is locked");
        assert!(container.get_item(0).is(&vanilla_items::DIAMOND));
        assert!(container.get_item(26).is(&vanilla_items::STONE));
        assert_eq!(container.get_item(26).count(), 3);
    }

    /// Loads `CustomName`/`lock` into a chest the way `/data merge` or a
    /// placed block item would.
    fn load_identity(chest: &ChestBlockEntity, name: Option<&str>, lock_item: Option<&str>) {
        let mut nbt = NbtCompound::new();
        if let Some(name) = name {
            nbt.insert(
                "CustomName",
                TextComponent::plain(name.to_owned()).to_codec_nbt(),
            );
        }
        if let Some(lock_item) = lock_item {
            let mut lock = NbtCompound::new();
            lock.insert("items", lock_item);
            nbt.insert("lock", lock);
        }
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("identity NBT reads");
        chest.load_additional(&borrowed);
    }

    fn chest_entity(fixture: &TestWorld) -> Arc<dyn BlockEntity> {
        fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity")
    }

    #[test]
    fn a_locked_chest_refuses_to_open_without_the_key() {
        let (fixture, player) = chest_fixture("chest_locked");
        let block_entity = chest_entity(&fixture);
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        load_identity(chest, None, Some("minecraft:diamond"));

        assert_eq!(use_chest(&fixture, &player), InteractionResult::Success);
        assert!(!player.has_container_open());
        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn a_locked_chest_opens_with_the_key_in_the_main_hand() {
        let (fixture, player) = chest_fixture("chest_locked_key");
        let block_entity = chest_entity(&fixture);
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        load_identity(chest, None, Some("minecraft:diamond"));
        player.inventory.lock().set_item_in_hand(
            InteractionHand::MainHand,
            ItemStack::new(&vanilla_items::DIAMOND),
        );

        assert_eq!(use_chest(&fixture, &player), InteractionResult::Success);
        assert!(player.has_container_open());
        assert_eq!(open_count(&fixture), 1);
    }

    #[test]
    fn a_spectator_opens_a_locked_chest_without_the_key() {
        let (fixture, player) = chest_fixture("chest_locked_spectator");
        let block_entity = chest_entity(&fixture);
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        load_identity(chest, None, Some("minecraft:diamond"));
        player.restore_game_modes(GameType::Spectator, None);

        assert_eq!(use_chest(&fixture, &player), InteractionResult::Success);
        assert!(player.has_container_open());
        // Vanilla `ChestBlockEntity.startOpen` ignores spectators.
        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn the_custom_name_and_lock_survive_an_nbt_round_trip() {
        let (fixture, _player) = chest_fixture("chest_identity_round_trip");
        let block_entity = chest_entity(&fixture);
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        load_identity(chest, Some("Storage"), Some("minecraft:diamond"));

        let mut nbt = NbtCompound::new();
        block_entity.save_additional(&mut nbt);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("chest NBT decodes");

        let loaded =
            ChestBlockEntity::new(Weak::new(), TEST_POS, vanilla_blocks::CHEST.default_state());
        loaded.load_additional(&borrowed);

        assert!(loaded.identity().is_locked());
        assert_eq!(
            loaded
                .identity()
                .display_name(|| TextComponent::translated(translations::CONTAINER_CHEST.msg())),
            TextComponent::plain("Storage"),
        );
    }

    #[test]
    fn the_chest_contents_round_trip_through_the_container_component() {
        let (fixture, _player) = chest_fixture("chest_component_round_trip");
        let block_entity = chest_entity(&fixture);
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        {
            let container_ref = block_entity
                .container_ref()
                .expect("a chest exposes its container");
            let mut guard = ContainerLockGuard::lock_all(&[&container_ref]);
            guard
                .get_mut(container_ref.container_id())
                .expect("the chest container is locked")
                .set_item(4, ItemStack::with_count(&vanilla_items::DIAMOND, 7));
        }

        let mut components = DataComponentMap::new();
        chest.collect_implicit_components(&mut components);
        let contents = components
            .get(CONTAINER)
            .expect("a chest publishes its contents as a component");

        let placed =
            ChestBlockEntity::new(Weak::new(), TEST_POS, vanilla_blocks::CHEST.default_state());
        let mut item = ItemStack::new(&vanilla_items::CHEST);
        item.set(
            CONTAINER,
            contents,
        );
        assert_eq!(
            item.get(CONTAINER)
                .map(|c| c.items().len()),
            Some(5),
            "the placed stack carries the container component"
        );
        placed.apply_implicit_components(&item);

        let container_ref = placed
            .container_ref()
            .expect("a chest exposes its container");
        let guard = ContainerLockGuard::lock_all(&[&container_ref]);
        let container = guard
            .get(container_ref.container_id())
            .expect("the placed chest container is locked");
        assert!(container.get_item(4).is(&vanilla_items::DIAMOND));
        assert_eq!(container.get_item(4).count(), 7);
    }
}
