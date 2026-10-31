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
        menu::kinds::{ShulkerBoxKind, shulker_box_with_kind},
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
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Success;
        };
        let Some(shulker) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>() else {
            return InteractionResult::Success;
        };
        let Some(container_ref) = shulker.container_ref() else {
            return InteractionResult::Success;
        };
        if !shulker.can_open(state, world, pos) {
            return InteractionResult::Success;
        }

        let display_name = shulker
            .identity()
            .display_name(|| TextComponent::translated(translations::CONTAINER_SHULKER_BOX.msg()));
        // `BaseContainerBlockEntity.createMenu` refuses to build a menu for
        // a locked box after telling the player which item unlocks it.
        if !shulker.identity().can_open(player) {
            player.close_container();
            ContainerIdentity::send_locked_notification(world, player, pos, display_name);
            player.award_custom_stat(&vanilla_custom_stats::OPEN_SHULKER_BOX);
            return InteractionResult::Success;
        }

        // `RandomizableContainerBlockEntity.createMenu` rolls the pending
        // loot table before the menu exists.
        shulker.unpack_loot_table(Some(player));

        let inventory = Arc::clone(&player.inventory);
        player.open_menu(display_name, move |context| {
            // `ShulkerBoxMenu` calls `container.startOpen(player)` in its
            // constructor, after the previous menu has been closed.
            if let Some(shulker_box) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>() {
                shulker_box.start_open(context.player);
            }
            let container = container_ref.clone();
            shulker_box_with_kind(
                inventory,
                context.container_id,
                container.clone(),
                ShulkerBoxKind::new(container).with_close_hook(move |player| {
                    if let Some(shulker_box) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>()
                    {
                        shulker_box.stop_open(player);
                    }
                }),
            )
        });

        player.award_custom_stat(&vanilla_custom_stats::OPEN_SHULKER_BOX);
        // TODO: Anger nearby piglins (PiglinAi.angerNearbyPiglins)

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
            // Vanilla rolls the pending table so the box's loot is not lost
            // when the block drops through its loot table.
            shulker_box_block_entity.unpack_loot_table(Some(player));
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
        let Some(block_entity) = world.get_block_entity(pos) else {
            return 0;
        };
        // `AbstractContainerMenu.getRedstoneSignalFromContainer` reads every
        // slot, which unpacks a pending loot table first.
        if let Some(shulker_box) = block_entity.downcast_ref::<ShulkerBoxBlockEntity>() {
            shulker_box.unpack_loot_table(None);
        }
        let Some(container_ref) = ContainerRef::from_block_entity(block_entity) else {
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

    use glam::{DVec3, IVec3};
    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_protocol::packets::game::SoundSource;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{sound_events, vanilla_blocks, vanilla_items};
    use vecta_utils::types::{GameType, InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::block_entity::SharedBlockEntity;
    use crate::bootstrap::init_globals;
    use crate::entity::{Entity as _, next_entity_id};
    use crate::player::ResetReason;
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
        played_sounds, recording_connection,
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

    fn shulker_box(fixture: &TestWorld) -> SharedBlockEntity {
        fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed shulker box has a block entity")
    }

    fn open_count(fixture: &TestWorld) -> i32 {
        shulker_box(fixture)
            .downcast_ref::<ShulkerBoxBlockEntity>()
            .expect("the shulker box block must create its block entity")
            .open_count()
    }

    fn animation_status(fixture: &TestWorld) -> AnimationStatus {
        shulker_box(fixture)
            .downcast_ref::<ShulkerBoxBlockEntity>()
            .expect("the shulker box block must create its block entity")
            .animation_status()
    }

    /// A shulker box fixture whose viewer receives packets, so the sound the
    /// server plays can be inspected instead of assumed.
    fn listening_shulker_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::SHULKER_BOX.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "ShulkerListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the box's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the sound packet to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    /// `ShulkerBoxBlockEntity.startOpen`/`stopOpen` play at the block centre.
    const BLOCK_CENTRE_SOUND_POS: IVec3 = IVec3::new(36, 516, 36);

    #[test]
    fn opening_a_shulker_box_counts_the_viewer_and_starts_the_lid() {
        let (fixture, player) = shulker_fixture("shulker_open");
        assert_eq!(animation_status(&fixture), AnimationStatus::Closed);

        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);

        assert_eq!(open_count(&fixture), 1);
        // `startOpen` calls `blockEvent(1, openCount)`, which is what starts the
        // lid animation; Vecta runs the queue on the next block-event pass.
        fixture.world.run_block_events();
        assert!(matches!(
            animation_status(&fixture),
            AnimationStatus::Opening
        ));

        player.close_container();
        assert_eq!(open_count(&fixture), 0);
        fixture.world.run_block_events();
        assert!(matches!(
            animation_status(&fixture),
            AnimationStatus::Closing
        ));
    }

    #[test]
    fn opening_plays_the_shulker_box_open_sound() {
        let (fixture, player, packets) = listening_shulker_fixture("shulker_open_sound");

        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);

        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "opening a box plays exactly one sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_SHULKER_BOX_OPEN.key);
        assert_eq!(sounds[0].1, SoundSource::Blocks.as_varint());
        assert_eq!(sounds[0].2, BLOCK_CENTRE_SOUND_POS);
        assert!((sounds[0].3 - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn closing_plays_the_shulker_box_close_sound() {
        let (fixture, player, packets) = listening_shulker_fixture("shulker_close_sound");
        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);
        packets.lock().clear();

        player.close_container();

        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "closing a box plays exactly one sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_SHULKER_BOX_CLOSE.key);
        assert_eq!(sounds[0].1, SoundSource::Blocks.as_varint());
        assert_eq!(sounds[0].2, BLOCK_CENTRE_SOUND_POS);
    }

    #[test]
    fn the_second_viewer_of_two_does_not_replay_the_open_sound() {
        // `startOpen` only plays the sound and fires the game event when the
        // count reaches one.
        let (fixture, player, packets) = listening_shulker_fixture("shulker_two_viewers");
        let second = TestPlayerBuilder::new(Arc::clone(&fixture.world), "ShulkerSecond", 2).build();
        second.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));

        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);
        packets.lock().clear();
        assert_eq!(use_shulker(&fixture, &second), InteractionResult::Success);

        assert_eq!(open_count(&fixture), 2);
        assert!(
            played_sounds(&packets).is_empty(),
            "the second viewer does not re-open the box"
        );

        second.close_container();
        assert_eq!(open_count(&fixture), 1, "one viewer is still there");

        player.close_container();
        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn a_spectator_does_not_count_as_a_viewer() {
        // `ShulkerBoxBlockEntity.startOpen` returns early for spectators.
        let (fixture, player) = shulker_fixture("shulker_spectator");
        player.restore_game_modes(GameType::Spectator, None);

        assert_eq!(use_shulker(&fixture, &player), InteractionResult::Success);
        assert_eq!(open_count(&fixture), 0);
        assert_eq!(animation_status(&fixture), AnimationStatus::Closed);
    }
}
