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
    BlockStateProperties, ChestType, Direction, EnumProperty,
};
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, translations};

use crate::behavior::BlockEntityCreation;
use crate::behavior::InventoryAccess;
use crate::behavior::block::BlockBehavior;
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::BLOCK_ENTITIES;
use crate::block_entity::entities::ChestBlockEntity;
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::{ChestKind, chest_with_kind};
use crate::player::Player;
use crate::world::{LevelReader, World};

/// Number of rows a chest menu shows.
const CHEST_ROWS: usize = 3;

const FACING: &EnumProperty<Direction> = &BlockStateProperties::HORIZONTAL_FACING;

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
}

impl BlockBehavior for ChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // A single chest faces the player; joining two halves is a separate
        // placement step that also rewrites the neighbour's chest type.
        Some(
            self.block
                .default_state()
                .set_value(FACING, context.horizontal_direction().opposite())
                .set_value(&BlockStateProperties::CHEST_TYPE, ChestType::Single),
        )
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
            return InteractionResult::Pass;
        };
        let Some(container_ref) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return InteractionResult::Pass;
        };
        let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() else {
            return InteractionResult::Pass;
        };
        chest.player_open();

        let inventory = Arc::clone(&player.inventory);
        let closer = Arc::clone(&block_entity);
        player.open_menu(
            TextComponent::translated(translations::CONTAINER_CHEST.msg()),
            move |context| {
                let container = container_ref.clone();
                chest_with_kind(
                    inventory,
                    context.container_id,
                    container.clone(),
                    CHEST_ROWS,
                    ChestKind::new(container).with_close_hook(move |_| {
                        if let Some(chest) = closer.downcast_ref::<ChestBlockEntity>() {
                            chest.player_close();
                        }
                    }),
                )
            },
        );

        player.award_custom_stat(&vanilla_custom_stats::OPEN_CHEST);
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

    use glam::DVec3;
    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_registry::{item_stack::ItemStack, vanilla_blocks, vanilla_items};
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::block_entity::BlockEntity as _;
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
}
