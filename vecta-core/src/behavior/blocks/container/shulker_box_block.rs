use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::{
    block_entity_type::BlockEntityTypeRef,
    blocks::{
        BlockRef,
        block_state_ext::BlockStateExt,
        properties::{BlockStateProperties, EnumProperty},
        shapes::VoxelShape,
    },
    item_stack::ItemStack,
    items::item::BlockHitResult,
    vanilla_block_entity_types, vanilla_custom_stats,
};
use vecta_utils::{BlockLocalAabb, BlockPos, BlockStateId, Direction, Downcast, translations};

use crate::{
    behavior::{
        BlockBehavior, BlockCollisionBoxes, BlockCollisionContext, BlockEntityCreation,
        BlockLootContext, BlockPlaceContext, InteractionResult, InventoryAccess,
    },
    block_entity::{
        BLOCK_ENTITIES, BlockEntity, BlockEntityTicker,
        container_identity::ContainerIdentity,
        entities::{AnimationStatus, ShulkerBoxBlockEntity},
    },
    inventory::{
        container::calculate_redstone_signal_from_container,
        lock::{ContainerLockGuard, ContainerRef},
        menu::kinds::shulker_box,
    },
    player::Player,
    world::{LevelReader, World},
};

/// Behavior for shulker box blocks.
#[block_behavior]
pub struct ShulkerBoxBlock {
    block: BlockRef,
}

impl ShulkerBoxBlock {
    /// Face the shulker box's lid opens towards.
    pub const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING;

    /// Creates a new shulker block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

// Vanilla `ShulkerBoxBlock.SHAPES_OPEN_SUPPORT`, derived there with
// `Shapes.rotateAll(Block.boxZ(16.0, 0.0, 1.0))`. Rotation is not available at
// compile time here, so each rotated result is written out directly.
const OPEN_SUPPORT_DOWN_BOXES: &[BlockLocalAabb] =
    &[BlockLocalAabb::new(0.0, 0.0, 0.0, 1.0, 0.0625, 1.0)];
const OPEN_SUPPORT_UP_BOXES: &[BlockLocalAabb] =
    &[BlockLocalAabb::new(0.0, 0.9375, 0.0, 1.0, 1.0, 1.0)];
const OPEN_SUPPORT_NORTH_BOXES: &[BlockLocalAabb] =
    &[BlockLocalAabb::new(0.0, 0.0, 0.0, 1.0, 1.0, 0.0625)];
const OPEN_SUPPORT_SOUTH_BOXES: &[BlockLocalAabb] =
    &[BlockLocalAabb::new(0.0, 0.0, 0.9375, 1.0, 1.0, 1.0)];
const OPEN_SUPPORT_WEST_BOXES: &[BlockLocalAabb] =
    &[BlockLocalAabb::new(0.0, 0.0, 0.0, 0.0625, 1.0, 1.0)];
const OPEN_SUPPORT_EAST_BOXES: &[BlockLocalAabb] =
    &[BlockLocalAabb::new(0.9375, 0.0, 0.0, 1.0, 1.0, 1.0)];

/// The one-pixel slab covering `direction`'s face, all that still supports the
/// block once the lid has started opening.
const fn open_support_shape(direction: Direction) -> VoxelShape {
    VoxelShape::from_boxes(match direction {
        Direction::Down => OPEN_SUPPORT_DOWN_BOXES,
        Direction::Up => OPEN_SUPPORT_UP_BOXES,
        Direction::North => OPEN_SUPPORT_NORTH_BOXES,
        Direction::South => OPEN_SUPPORT_SOUTH_BOXES,
        Direction::West => OPEN_SUPPORT_WEST_BOXES,
        Direction::East => OPEN_SUPPORT_EAST_BOXES,
    })
}

impl BlockBehavior for ShulkerBoxBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        let facing = context.clicked_face();
        Some(
            self.block
                .default_state()
                .set_value(&BlockStateProperties::FACING, facing),
        )
    }

    fn use_without_item(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
        _inv: &mut InventoryAccess,
    ) -> InteractionResult {
        if let Some(block_entity) = world.get_block_entity(pos)
            && let Some(block_entity) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>()
            && let Some(container_ref) = block_entity.container_ref()
            && block_entity.can_open(state, world, pos)
        {
            let display_name = block_entity.identity().display_name(|| {
                TextComponent::translated(translations::CONTAINER_SHULKER_BOX.msg())
            });
            // `BaseContainerBlockEntity.createMenu` refuses to build a menu for
            // a locked box after telling the player which item unlocks it.
            if !block_entity.identity().can_open(player) {
                player.close_container();
                ContainerIdentity::send_locked_notification(world, player, pos, display_name);
                player.award_custom_stat(&vanilla_custom_stats::OPEN_SHULKER_BOX);
                return InteractionResult::Success;
            }

            let inventory = Arc::clone(&player.inventory);
            player.open_menu(display_name, move |context| {
                shulker_box(inventory, context.container_id, container_ref)
            });

            player.award_custom_stat(&vanilla_custom_stats::OPEN_SHULKER_BOX);
            // TODO: Anger nearby piglins (PiglinAi.angerNearbyPiglins)
        }
        InteractionResult::Success
    }

    fn player_will_destroy(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
    ) -> BlockStateId {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return state;
        };

        let Some(shulker_box_block_entity) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>()
        else {
            return state;
        };

        if player.prevents_block_drops() && !shulker_box_block_entity.is_empty() {
            let item = shulker_box_block_entity.shulker_box_as_item(state);
            world.pop_resource(pos, item);
        } else {
            // TODO: maybe? shulkerBoxBlockEntity.unpackLootTable(player);
        }

        state
    }

    fn get_drops(
        &self,
        state: BlockStateId,
        context: &BlockLootContext<'_>,
    ) -> Option<Vec<ItemStack>> {
        let block_entity = context.block_entity()?;
        let shulker_box_block_entity = block_entity.downcast_ref::<ShulkerBoxBlockEntity>()?;

        let item = shulker_box_block_entity.shulker_box_as_item(state);
        Some(vec![item])
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::SHULKER_BOX,
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
        BlockEntityTicker::for_matching_entity_tick(
            block_entity_type,
            &vanilla_block_entity_types::SHULKER_BOX,
        )
    }

    fn trigger_event(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        event: i32,
        data: i32,
    ) -> bool {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return false;
        };
        block_entity.trigger_event(event, data)
    }

    /// Vanilla `ShulkerBoxBlock.getShape`, which the collision shape inherits.
    ///
    /// The lid box is computed from live animation progress, so the boxes are
    /// produced directly instead of going through a static [`VoxelShape`].
    fn get_collision_boxes(
        &self,
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        _context: BlockCollisionContext,
    ) -> BlockCollisionBoxes {
        if let Some(block_entity) = world.get_block_entity(pos)
            && let Some(shulker_box_block_entity) =
                block_entity.downcast_ref::<ShulkerBoxBlockEntity>()
        {
            return BlockCollisionBoxes::from_slice(&[
                shulker_box_block_entity.get_bounding_box(state)
            ]);
        }

        BlockCollisionBoxes::from_slice(VoxelShape::FULL_BLOCK.boxes())
    }

    fn get_block_support_boxes(
        &self,
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
    ) -> BlockCollisionBoxes {
        let shape = if let Some(block_entity) = world.get_block_entity(pos)
            && let Some(shulker_box_block_entity) =
                block_entity.downcast_ref::<ShulkerBoxBlockEntity>()
            && !matches!(
                shulker_box_block_entity.animation_status(),
                AnimationStatus::Closed
            ) {
            open_support_shape(state.get_value(Self::FACING).opposite())
        } else {
            VoxelShape::FULL_BLOCK
        };

        BlockCollisionBoxes::from_slice(shape.boxes())
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

    fn fits_inside_container_items(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use glam::DVec3;
    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{vanilla_blocks, vanilla_items};
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::bootstrap::init_globals;
    use crate::entity::Entity as _;
    use crate::test_support::{
        TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    fn shulker_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::SHULKER_BOX.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "ShulkerUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn shulker_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn use_shulker(fixture: &TestWorld, player: &Arc<Player>) -> InteractionResult {
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        ShulkerBoxBlock::new(&vanilla_blocks::SHULKER_BOX).use_without_item(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
            player,
            &shulker_hit(),
            &mut inventory,
        )
    }

    fn lock_shulker(fixture: &TestWorld, lock_item: &str) {
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed shulker box has a block entity");
        let shulker_box = block_entity
            .downcast_ref::<ShulkerBoxBlockEntity>()
            .expect("the shulker box block must create its block entity");
        let mut nbt = NbtCompound::new();
        let mut lock = NbtCompound::new();
        lock.insert("items", lock_item);
        nbt.insert("lock", lock);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("lock NBT");
        shulker_box.load_additional(&borrowed);
    }

    #[test]
    fn a_locked_shulker_box_refuses_to_open_without_the_key() {
        let (fixture, player) = shulker_fixture("shulker_locked");
        lock_shulker(&fixture, "minecraft:diamond");

        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);
        assert!(!player.has_container_open());
    }

    #[test]
    fn a_locked_shulker_box_opens_with_the_key_in_the_main_hand() {
        let (fixture, player) = shulker_fixture("shulker_locked_key");
        lock_shulker(&fixture, "minecraft:diamond");
        player.inventory.lock().set_item_in_hand(
            InteractionHand::MainHand,
            ItemStack::new(&vanilla_items::DIAMOND),
        );

        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);
        assert!(player.has_container_open());
    }
}
