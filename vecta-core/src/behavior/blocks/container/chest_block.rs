//! Chest block behavior implementation.
//!
//! A chest opens a 27-slot container menu, tracks its viewers so clients can
//! animate the lid, and exposes its fullness to comparators. Two adjacent chests
//! facing the same way join into one 54-slot double chest.

use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, ChestType, Direction, EnumProperty,
};
use vecta_registry::stat::custom::CustomStatRef;
use vecta_registry::vanilla_block_tags::BlockTag;
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, translations};

use crate::behavior::BlockEntityCreation;
use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, schedule_water_tick_if_waterlogged};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::ChestBlockEntity;
use crate::block_entity::{BLOCK_ENTITIES, BlockEntity as _, SharedBlockEntity};
use crate::entity::Entity as _;
use crate::inventory::container::{Container, calculate_redstone_signal_from_containers};
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::{ChestKind, chest_parts_with_kind};
use crate::player::Player;
use crate::world::{LevelReader, ScheduledTickAccess, World, is_redstone_conductor};

/// Number of rows one chest half fills.
const CHEST_ROWS: usize = 3;

const FACING: &EnumProperty<Direction> = &BlockStateProperties::HORIZONTAL_FACING;
const CHEST_TYPE: &EnumProperty<ChestType> = &BlockStateProperties::CHEST_TYPE;
const WATERLOGGED: &BoolProperty = &BlockStateProperties::WATERLOGGED;

/// Vanilla `DoubleBlockCombiner.BlockType`: which part of a joined pair a state is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChestHalf {
    /// Not joined to another chest.
    Single,
    /// The half that fills the first slots of the joined container.
    First,
    /// The half that fills the last slots.
    Second,
}

/// Vanilla `DoubleBlockCombiner.NeighborCombineResult` for a chest.
pub(crate) enum ChestPair {
    /// No block entity, or a half is covered so the chest cannot be used.
    None,
    /// A chest on its own.
    Single(SharedBlockEntity),
    /// Both halves of a double chest, in joined-container slot order.
    Double(SharedBlockEntity, SharedBlockEntity),
}

/// Which neighbours a chest joins, i.e. vanilla `ChestBlock.chestCanConnectTo`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChestConnect {
    /// Vanilla `ChestBlock.chestCanConnectTo`: only the very same block joins,
    /// so a plain chest and a trapped chest never pair up.
    SameBlock,
    /// Vanilla `CopperChestBlock.chestCanConnectTo`: any block in the
    /// `#minecraft:copper_chests` tag that still has a chest type, so two copper
    /// chests join even when their oxidation differs.
    CopperChests,
}

impl ChestConnect {
    /// Returns whether a chest of `block` may join the chest state `neighbor`.
    pub(crate) fn connects(self, block: BlockRef, neighbor: BlockStateId) -> bool {
        match self {
            Self::SameBlock => neighbor.get_block() == block,
            Self::CopperChests => {
                neighbor.get_block().has_tag(&BlockTag::COPPER_CHESTS)
                    && neighbor.try_get_value(CHEST_TYPE).is_some()
            }
        }
    }
}

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

    /// Vanilla `ChestBlock.isChestBlockedAt`: a redstone conductor above the
    /// chest keeps it shut. The sitting-cat half needs cats, which Vecta lacks.
    fn is_blocked(level: &dyn LevelReader, pos: BlockPos) -> bool {
        let above = pos.above();
        is_redstone_conductor(level, level.get_block_state(above), above)
    }

    /// Vanilla `ChestBlock.getBlockType`.
    fn half(state: BlockStateId) -> ChestHalf {
        match state.get_value(CHEST_TYPE) {
            ChestType::Single => ChestHalf::Single,
            ChestType::Right => ChestHalf::First,
            ChestType::Left => ChestHalf::Second,
        }
    }

    /// Vanilla `ChestBlock.getConnectedDirection`: the direction from this half
    /// toward its partner.
    #[must_use]
    pub fn get_connected_direction(state: BlockStateId) -> Direction {
        let facing = state.get_value(FACING);
        if state.get_value(CHEST_TYPE) == ChestType::Left {
            facing.rotate_y_clockwise()
        } else {
            facing.rotate_y_counter_clockwise()
        }
    }

    /// Vanilla `ChestBlock.candidatePartnerFacing`: the facing of a lone chest
    /// in `neighbour_direction`, which this chest may join.
    fn candidate_partner_facing(
        connect: ChestConnect,
        block: BlockRef,
        level: &World,
        pos: BlockPos,
        neighbour_direction: Direction,
    ) -> Option<Direction> {
        let state = level.get_block_state(pos.relative(neighbour_direction));
        (connect.connects(block, state) && state.get_value(CHEST_TYPE) == ChestType::Single)
            .then(|| state.get_value(FACING))
    }

    /// Vanilla `ChestBlock.getChestType`: the half this chest becomes when it is
    /// placed next to a lone chest facing `facing`.
    fn get_chest_type(
        connect: ChestConnect,
        block: BlockRef,
        level: &World,
        pos: BlockPos,
        facing: Direction,
    ) -> ChestType {
        if Self::candidate_partner_facing(connect, block, level, pos, facing.rotate_y_clockwise())
            == Some(facing)
        {
            ChestType::Left
        } else if Self::candidate_partner_facing(
            connect,
            block,
            level,
            pos,
            facing.rotate_y_counter_clockwise(),
        ) == Some(facing)
        {
            ChestType::Right
        } else {
            ChestType::Single
        }
    }

    /// Vanilla `ChestBlock.getStateForPlacement`.
    pub(crate) fn placement_state(
        block: BlockRef,
        context: &BlockPlaceContext<'_>,
        connect: ChestConnect,
    ) -> BlockStateId {
        let pos = context.place_pos();
        let level = context.world;
        let mut facing = context.horizontal_direction().opposite();
        let mut chest_type = ChestType::Single;
        let secondary_use = context.is_secondary_use_active();
        let clicked_face = context.clicked_face();

        // Sneaking against a horizontal face joins that chest's front instead of
        // turning to face the player.
        if clicked_face.is_horizontal()
            && secondary_use
            && let Some(partner_facing) =
                Self::candidate_partner_facing(connect, block, level, pos, clicked_face.opposite())
            && partner_facing.get_axis() != clicked_face.get_axis()
        {
            facing = partner_facing;
            chest_type = if facing.rotate_y_counter_clockwise() == clicked_face.opposite() {
                ChestType::Right
            } else {
                ChestType::Left
            };
        }

        if chest_type == ChestType::Single && !secondary_use {
            chest_type = Self::get_chest_type(connect, block, level, pos, facing);
        }

        block
            .default_state()
            .set_value(FACING, facing)
            .set_value(CHEST_TYPE, chest_type)
            .set_value(WATERLOGGED, context.is_water_source())
    }

    /// Vanilla `ChestBlock.updateShape`.
    pub(crate) fn shape_state(
        block: BlockRef,
        state: BlockStateId,
        world: &dyn ScheduledTickAccess,
        pos: BlockPos,
        direction: Direction,
        neighbor_state: BlockStateId,
        connect: ChestConnect,
    ) -> BlockStateId {
        schedule_water_tick_if_waterlogged(state, world, pos);

        if connect.connects(block, neighbor_state) && direction.is_horizontal() {
            let neighbour_type = neighbor_state.get_value(CHEST_TYPE);
            if state.get_value(CHEST_TYPE) == ChestType::Single
                && neighbour_type != ChestType::Single
                && state.get_value(FACING) == neighbor_state.get_value(FACING)
                && Self::get_connected_direction(neighbor_state) == direction.opposite()
            {
                return state.set_value(CHEST_TYPE, neighbour_type.opposite());
            }
        } else if Self::get_connected_direction(state) == direction {
            return state.set_value(CHEST_TYPE, ChestType::Single);
        }

        state
    }

    /// Vanilla `ChestBlock.combine`: resolves the chest at `pos` into the block
    /// entities that back it, refusing a half that is covered.
    pub(crate) fn combine(
        state: BlockStateId,
        level: &dyn LevelReader,
        pos: BlockPos,
    ) -> ChestPair {
        let Some(block_entity) = level.get_block_entity(pos) else {
            return ChestPair::None;
        };
        if Self::is_blocked(level, pos) {
            return ChestPair::None;
        }
        let half = Self::half(state);
        if half == ChestHalf::Single {
            return ChestPair::Single(block_entity);
        }

        let neighbour_pos = pos.relative(Self::get_connected_direction(state));
        let neighbour_state = level.get_block_state(neighbour_pos);
        if neighbour_state.get_block() != state.get_block() {
            return ChestPair::Single(block_entity);
        }
        let neighbour_half = Self::half(neighbour_state);
        if neighbour_half == ChestHalf::Single
            || neighbour_half == half
            || neighbour_state.get_value(FACING) != state.get_value(FACING)
        {
            return ChestPair::Single(block_entity);
        }
        if Self::is_blocked(level, neighbour_pos) {
            return ChestPair::None;
        }
        let Some(neighbour) = level.get_block_entity(neighbour_pos) else {
            return ChestPair::Single(block_entity);
        };

        if half == ChestHalf::First {
            ChestPair::Double(block_entity, neighbour)
        } else {
            ChestPair::Double(neighbour, block_entity)
        }
    }

    /// Opens the chest menu over `halves`, which are listed in joined-container
    /// slot order.
    fn open_menu(
        world: &Arc<World>,
        player: &Player,
        halves: &[SharedBlockEntity],
        open_stat: CustomStatRef,
    ) -> InteractionResult {
        let mut chests = Vec::with_capacity(halves.len());
        for block_entity in halves {
            let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() else {
                return InteractionResult::Success;
            };
            chests.push(chest);
        }
        let Some(first) = chests.first() else {
            return InteractionResult::Success;
        };

        let default_name = if chests.len() == 1 {
            translations::CONTAINER_CHEST.msg()
        } else {
            translations::CONTAINER_CHEST_DOUBLE.msg()
        };
        let display_name = chests
            .iter()
            .find_map(|chest| chest.identity().custom_name())
            .unwrap_or_else(|| TextComponent::translated(default_name));

        // Vanilla `BaseContainerBlockEntity.createMenu` refuses a locked
        // container after telling the player which item unlocks it, and the
        // previous menu is already closed by then.
        if !chests.iter().all(|chest| chest.can_open(player)) {
            player.close_container();
            if !player.is_spectator() {
                let mut sound_pos = centre_of(first.get_block_pos());
                if chests.len() > 1 {
                    // Vanilla reports a double chest from between its halves.
                    let (dx, _, dz) =
                        Self::get_connected_direction(first.get_block_state()).offset();
                    sound_pos.x += f64::from(dx) * 0.5;
                    sound_pos.z += f64::from(dz) * 0.5;
                }
                ContainerIdentity::send_locked_notification_at(
                    world,
                    player,
                    sound_pos,
                    display_name,
                );
            }
            player.award_custom_stat(open_stat);
            return InteractionResult::Success;
        }

        let mut containers = Vec::with_capacity(halves.len());
        for block_entity in halves {
            // `RandomizableContainerBlockEntity.createMenu` rolls the pending
            // loot table before the menu exists, with the opener as context.
            if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
                chest.unpack_loot_table(Some(player));
            }
            let Some(container_ref) = ContainerRef::from_block_entity(Arc::clone(block_entity))
            else {
                return InteractionResult::Success;
            };
            containers.push(container_ref);
        }

        let rows = chests.len() * CHEST_ROWS;
        let inventory = Arc::clone(&player.inventory);
        let halves: Arc<Vec<SharedBlockEntity>> = Arc::new(halves.to_vec());
        let starting = Arc::clone(&halves);
        let closing = Arc::clone(&halves);
        player.open_menu(display_name, move |context| {
            // `ChestMenu` starts every opener in its constructor, after the
            // previous menu has been closed.
            for block_entity in starting.iter() {
                if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
                    chest.start_open(context.player);
                }
            }
            let menu_containers = containers.clone();
            let kind = ChestKind::halves(containers).with_close_hook(move |player| {
                for block_entity in closing.iter() {
                    if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
                        chest.stop_open(player);
                    }
                }
            });
            chest_parts_with_kind(
                inventory,
                context.container_id,
                &menu_containers,
                rows,
                kind,
            )
        });

        player.award_custom_stat(open_stat);
        // TODO: Anger nearby piglins (PiglinAi.angerNearbyPiglins) once piglin AI exists.
        InteractionResult::Success
    }

    /// Vanilla `ChestBlock.useWithoutItem` for the chest at `pos`, awarding
    /// `open_stat` to the player.
    pub(crate) fn open_at(
        world: &Arc<World>,
        player: &Player,
        state: BlockStateId,
        pos: BlockPos,
        open_stat: CustomStatRef,
    ) -> InteractionResult {
        // Vanilla returns SUCCESS whether or not a menu opens.
        let halves = match Self::combine(state, world.as_ref(), pos) {
            ChestPair::None => return InteractionResult::Success,
            ChestPair::Single(block_entity) => vec![block_entity],
            ChestPair::Double(first, second) => vec![first, second],
        };
        Self::open_menu(world, player, &halves, open_stat)
    }

    /// Vanilla `ChestBlock.tick`: recounts the viewers of an open chest.
    pub(crate) fn recheck_at(world: &Arc<World>, pos: BlockPos) {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
            chest.recheck_open();
        }
    }

    /// Vanilla `BaseEntityBlock.triggerEvent`: the block entity decides whether
    /// the event is published to clients.
    pub(crate) fn dispatch_block_event(
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

    /// Vanilla `ChestBlock.getAnalogOutputSignal`: the fullness of both halves
    /// of the joined container.
    pub(crate) fn analog_output(
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
    ) -> i32 {
        let halves = match Self::combine(state, world, pos) {
            ChestPair::None => return 0,
            ChestPair::Single(block_entity) => vec![block_entity],
            ChestPair::Double(first, second) => vec![first, second],
        };

        let mut containers = Vec::with_capacity(halves.len());
        for block_entity in &halves {
            // `AbstractContainerMenu.getRedstoneSignalFromContainer` reads every
            // slot, which unpacks a pending loot table first.
            if let Some(chest) = block_entity.downcast_ref::<ChestBlockEntity>() {
                chest.unpack_loot_table(None);
            }
            let Some(container_ref) = ContainerRef::from_block_entity(Arc::clone(block_entity))
            else {
                return 0;
            };
            containers.push(container_ref);
        }

        let guard = ContainerLockGuard::lock_all(&containers);
        let locked: Vec<&dyn Container> = containers
            .iter()
            .filter_map(|container| guard.get(container.container_id()))
            .collect();
        calculate_redstone_signal_from_containers(locked)
    }
}

/// Centre of a block, which is where Vanilla places container sounds.
fn centre_of(pos: BlockPos) -> glam::DVec3 {
    glam::DVec3::new(
        f64::from(pos.x()) + 0.5,
        f64::from(pos.y()) + 0.5,
        f64::from(pos.z()) + 0.5,
    )
}

impl BlockBehavior for ChestBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(Self::placement_state(
            self.block,
            context,
            ChestConnect::SameBlock,
        ))
    }

    fn update_shape(
        &self,
        state: BlockStateId,
        world: &dyn ScheduledTickAccess,
        pos: BlockPos,
        direction: Direction,
        _neighbor_pos: BlockPos,
        neighbor_state: BlockStateId,
    ) -> BlockStateId {
        Self::shape_state(
            self.block,
            state,
            world,
            pos,
            direction,
            neighbor_state,
            ChestConnect::SameBlock,
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
        Self::open_at(world, player, state, pos, &vanilla_custom_stats::OPEN_CHEST)
    }

    fn tick(&self, _state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        Self::recheck_at(world, pos);
    }

    /// Vanilla `BaseEntityBlock.triggerEvent`, which `ChestBlock` inherits: the
    /// block entity decides whether the event is published to clients.
    fn trigger_event(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        event: i32,
        data: i32,
    ) -> bool {
        Self::dispatch_block_event(world, pos, event, data)
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

    /// Vanilla `ChestBlock.affectNeighborsAfterRemoval`, which defers to
    /// `Containers.updateNeighboursAfterDestroy`: a removed chest must make the
    /// comparator that read it re-evaluate.
    fn affect_neighbors_after_removal(
        &self,
        _state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        world.update_neighbor_for_output_signal(pos, self.block);
    }

    fn get_analog_output_signal(
        &self,
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        _direction: Direction,
    ) -> i32 {
        Self::analog_output(state, world, pos)
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
    use vecta_registry::{
        RegistryEntry as _, item_stack::ItemStack, sound_events, vanilla_blocks, vanilla_items,
        vanilla_menu_types,
    };
    use vecta_utils::types::{GameType, InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::behavior::context::{BlockPlaceContext, PlacementOrientation, PlacementSource};
    use crate::block_entity::BlockEntity;
    use crate::bootstrap::init_globals;
    use crate::entity::{SharedEntity, next_entity_id};
    use crate::player::ResetReason;
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, block_events, fresh_test_world,
        insert_ready_full_chunk, opened_screens, played_sounds, recording_connection,
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

    /// Registers the fixture player in the world so entity queries (the opener
    /// recount) can find it.
    fn register_player(fixture: &TestWorld, player: &Arc<Player>) {
        let player_arc: Arc<Player> = Arc::clone(player);
        let entity: SharedEntity = player_arc;
        fixture
            .world
            .try_add_entity(entity)
            .expect("the test chunk is loaded, so the player registers");
    }

    /// A chest fixture whose viewer receives packets, so what the server sends
    /// can be inspected instead of assumed.
    fn listening_chest_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "ChestListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the chest's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the block event to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
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
    fn opening_a_chest_broadcasts_the_lid_event_to_clients() {
        // `ContainerOpenersCounter.openerCountChanged` sends
        // `level.blockEvent(pos, block, 1, current)`, and the client animates the
        // lid from that event.
        let (fixture, player, packets) = listening_chest_fixture("chest_lid_event");

        assert_eq!(use_chest(&fixture, &player), InteractionResult::Success);
        fixture.world.run_block_events();

        let events = block_events(&packets);
        assert_eq!(events.len(), 1, "opening sends exactly one block event");
        assert_eq!(events[0].0, TEST_POS);
        assert_eq!(events[0].1, 1, "event id 1 is the lid event");
        assert_eq!(events[0].2, 1, "the parameter carries the viewer count");
        assert_eq!(events[0].3, vanilla_blocks::CHEST.id() as i32);
    }

    #[test]
    fn a_recheck_keeps_a_viewer_that_is_still_there() {
        let (fixture, player) = chest_fixture("chest_recheck_keeps");
        register_player(&fixture, &player);
        use_chest(&fixture, &player);
        assert_eq!(open_count(&fixture), 1);

        let block_entity = chest_entity(&fixture);
        block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity")
            .recheck_open();

        assert_eq!(open_count(&fixture), 1);
    }

    #[test]
    fn a_recheck_drops_a_viewer_that_left_the_container() {
        let (fixture, player) = chest_fixture("chest_recheck_drops");
        register_player(&fixture, &player);
        use_chest(&fixture, &player);
        assert_eq!(open_count(&fixture), 1);

        // The menu is still open, but the player walked out of reach, which is
        // exactly the case the scheduled recount exists for.
        player
            .base()
            .try_set_position(DVec3::new(15.5, 64.0, 15.5))
            .expect("the player walks out of range");
        let block_entity = chest_entity(&fixture);
        block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity")
            .recheck_open();

        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn the_block_tick_recounts_the_viewers() {
        let (fixture, player) = chest_fixture("chest_recheck_tick");
        register_player(&fixture, &player);
        use_chest(&fixture, &player);
        assert_eq!(open_count(&fixture), 1);
        player
            .base()
            .try_set_position(DVec3::new(15.5, 64.0, 15.5))
            .expect("the player walks out of range");

        ChestBlock::new(&vanilla_blocks::CHEST).tick(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
        );

        assert_eq!(open_count(&fixture), 0);
    }

    #[test]
    fn a_loot_backed_chest_rolls_once_and_saves_the_table_instead_of_items() {
        let (fixture, _player) = chest_fixture("chest_loot");
        let block_entity = chest_entity(&fixture);
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        let mut nbt = NbtCompound::new();
        nbt.insert("LootTable", "minecraft:chests/simple_dungeon");
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("loot NBT decodes");
        block_entity.load_additional(&borrowed);

        let mut pending = NbtCompound::new();
        block_entity.save_additional(&mut pending);
        assert!(
            pending.get("LootTable").is_some(),
            "an unrolled table is saved back"
        );
        assert!(
            pending.get("Items").is_none(),
            "a pending table replaces the item list"
        );

        chest.unpack_loot_table(None);

        let container_ref = block_entity
            .container_ref()
            .expect("a chest exposes its container");
        {
            let guard = ContainerLockGuard::lock_all(&[&container_ref]);
            let container = guard
                .get(container_ref.container_id())
                .expect("the chest container is locked");
            assert!(
                container.items().iter().any(|stack| !stack.is_empty()),
                "the dungeon table rolls at least one item"
            );
        }

        let mut rolled = NbtCompound::new();
        block_entity.save_additional(&mut rolled);
        assert!(
            rolled.get("LootTable").is_none(),
            "a consumed table is not written again"
        );
        assert!(
            rolled.get("Items").is_some(),
            "the rolled contents are persisted"
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
        item.set(CONTAINER, contents);
        assert_eq!(
            item.get(CONTAINER).map(|c| c.items().len()),
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

    /// Places a chest the way a player does, so the join rules see a real
    /// placement state and the neighbour gets a real shape update.
    fn place_chest(
        fixture: &TestWorld,
        pos: BlockPos,
        chest_facing: Direction,
        secondary_use: bool,
    ) {
        let behavior = ChestBlock::new(&vanilla_blocks::CHEST);
        let mut stack = ItemStack::new(&vanilla_items::CHEST);
        let source = PlacementSource::direct(
            None,
            InteractionHand::MainHand,
            &mut stack,
            PlacementOrientation::Directional {
                direction: chest_facing.opposite(),
            },
            secondary_use,
        );
        let hit = BlockHitResult {
            location: DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
            direction: Direction::Up,
            block_pos: pos,
            miss: false,
            inside: false,
            world_border_hit: false,
        };
        let context = BlockPlaceContext::new(&fixture.world, source, &hit);
        let state = behavior
            .get_state_for_placement(&context)
            .expect("a chest always has a placement state");
        assert!(fixture.world.set_block(pos, state, UpdateFlags::UPDATE_ALL));
    }

    /// Places two north-facing chests side by side, west half at [`TEST_POS`].
    fn double_chest_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "DoubleUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        place_chest(&fixture, TEST_POS, Direction::North, false);
        place_chest(&fixture, TEST_POS.east(), Direction::North, false);
        (fixture, player)
    }

    fn chest_type_at(fixture: &TestWorld, pos: BlockPos) -> ChestType {
        fixture.world.get_block_state(pos).get_value(CHEST_TYPE)
    }

    fn open_count_at(fixture: &TestWorld, pos: BlockPos) -> i32 {
        fixture
            .world
            .get_block_entity(pos)
            .expect("a placed chest has a block entity")
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity")
            .open_count()
    }

    fn use_chest_at(fixture: &TestWorld, player: &Arc<Player>, pos: BlockPos) -> InteractionResult {
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        ChestBlock::new(&vanilla_blocks::CHEST).use_without_item(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            player,
            &chest_hit(),
            &mut inventory,
        )
    }

    #[test]
    fn placing_a_chest_beside_a_lone_chest_joins_both_halves() {
        let (fixture, _player) = double_chest_fixture("chest_join_placement");

        // Vanilla joins the second chest on its own and turns the first into the
        // other half through the shape update the placement triggers.
        assert_eq!(chest_type_at(&fixture, TEST_POS), ChestType::Left);
        assert_eq!(chest_type_at(&fixture, TEST_POS.east()), ChestType::Right);
    }

    #[test]
    fn sneaking_against_a_chest_side_joins_that_chests_front() {
        init_globals();
        let fixture = fresh_test_world("chest_join_secondary_use");
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place_chest(&fixture, TEST_POS, Direction::North, false);

        // The player looks east, so a plain placement would face the new chest
        // west; sneaking against the lone chest's side overrides both.
        let behavior = ChestBlock::new(&vanilla_blocks::CHEST);
        let mut stack = ItemStack::new(&vanilla_items::CHEST);
        let source = PlacementSource::direct(
            None,
            InteractionHand::MainHand,
            &mut stack,
            PlacementOrientation::Directional {
                direction: Direction::East,
            },
            true,
        );
        let hit = BlockHitResult {
            location: DVec3::new(5.0, 64.5, 4.5),
            direction: Direction::East,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        };
        let context = BlockPlaceContext::new(&fixture.world, source, &hit);
        let state = behavior
            .get_state_for_placement(&context)
            .expect("a chest always has a placement state");
        assert_eq!(state.get_value(FACING), Direction::North);
        assert_eq!(state.get_value(CHEST_TYPE), ChestType::Right);
        assert!(
            fixture
                .world
                .set_block(TEST_POS.east(), state, UpdateFlags::UPDATE_ALL)
        );
        assert_eq!(chest_type_at(&fixture, TEST_POS), ChestType::Left);
    }

    #[test]
    fn removing_a_half_returns_the_other_to_a_single_chest() {
        let (fixture, _player) = double_chest_fixture("chest_split");

        assert!(fixture.world.set_block(
            TEST_POS.east(),
            vanilla_blocks::AIR.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        assert_eq!(chest_type_at(&fixture, TEST_POS), ChestType::Single);
    }

    #[test]
    fn a_double_chest_menu_shows_six_rows() {
        let (fixture, player, packets) = listening_double_chest_fixture("chest_double_rows");

        assert_eq!(
            use_chest_at(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );
        assert!(player.has_container_open());

        let screens = opened_screens(&packets);
        assert_eq!(screens.len(), 1, "opening a chest sends one open-screen");
        assert_eq!(
            screens[0].1,
            vanilla_menu_types::GENERIC_9X6.id() as i32,
            "a joined pair opens the six-row menu"
        );
    }

    #[test]
    fn a_double_chest_counts_the_viewer_on_both_halves() {
        let (fixture, player) = double_chest_fixture("chest_double_viewers");

        use_chest_at(&fixture, &player, TEST_POS);
        assert_eq!(open_count_at(&fixture, TEST_POS), 1);
        assert_eq!(open_count_at(&fixture, TEST_POS.east()), 1);

        player.close_container();
        assert_eq!(open_count_at(&fixture, TEST_POS), 0);
        assert_eq!(open_count_at(&fixture, TEST_POS.east()), 0);
    }

    #[test]
    fn opening_a_double_chest_announces_the_lid_once_from_the_right_half() {
        let (fixture, player, packets) = listening_double_chest_fixture("chest_double_lid");

        use_chest_at(&fixture, &player, TEST_POS);
        fixture.world.run_block_events();

        let events = block_events(&packets);
        assert_eq!(
            events.len(),
            2,
            "each half reports its own viewer count to clients"
        );
        assert!(events.iter().all(|event| event.1 == 1 && event.2 == 1));

        // Only the right half plays the sound, from the middle of the pair.
        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "a joined pair plays one lid sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_CHEST_OPEN.key);
        assert_eq!(sounds[0].2, glam::IVec3::new(40, 516, 36));
        assert_eq!(sounds[0].3, 0.5);
    }

    /// Double-chest fixture whose viewer receives packets.
    fn listening_double_chest_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place_chest(&fixture, TEST_POS, Direction::North, false);
        place_chest(&fixture, TEST_POS.east(), Direction::North, false);

        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "DoubleListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the chest's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the block event to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    #[test]
    fn the_comparator_reads_the_second_half_too() {
        let (fixture, _player) = double_chest_fixture("chest_double_comparator");
        let behavior = ChestBlock::new(&vanilla_blocks::CHEST);
        let state = fixture.world.get_block_state(TEST_POS);

        assert_eq!(
            behavior.get_analog_output_signal(state, &fixture.world, TEST_POS, Direction::South),
            0
        );

        // The west half is the second part of the joined container, so a
        // comparator that only read the clicked half would stay at zero.
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed chest has a block entity");
        let container_ref = block_entity
            .container_ref()
            .expect("a chest exposes its container");
        {
            let mut guard = ContainerLockGuard::lock_all(&[&container_ref]);
            guard
                .get_mut(container_ref.container_id())
                .expect("the chest container is locked")
                .set_item(0, ItemStack::with_count(&vanilla_items::STONE, 64));
        }

        // One full stack of 54 slots is under one comparator step.
        let signal =
            behavior.get_analog_output_signal(state, &fixture.world, TEST_POS, Direction::South);
        assert!(signal > 0, "the second half contributes to the signal");
    }

    #[test]
    fn a_covered_half_keeps_the_double_chest_shut() {
        let (fixture, player) = double_chest_fixture("chest_double_covered");
        assert!(fixture.world.set_block(
            TEST_POS.east().above(),
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        assert_eq!(
            use_chest_at(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );
        assert!(
            !player.has_container_open(),
            "vanilla refuses the whole pair when either half is covered"
        );
        assert_eq!(open_count_at(&fixture, TEST_POS), 0);
        assert_eq!(open_count_at(&fixture, TEST_POS.east()), 0);
    }

    #[test]
    fn a_locked_half_refuses_the_whole_double_chest() {
        let (fixture, player) = double_chest_fixture("chest_double_locked");
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS.east())
            .expect("a placed chest has a block entity");
        let chest = block_entity
            .downcast_ref::<ChestBlockEntity>()
            .expect("the chest block must create a chest block entity");
        load_identity(chest, None, Some("minecraft:diamond"));

        assert_eq!(
            use_chest_at(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );
        assert!(!player.has_container_open());
        assert_eq!(open_count_at(&fixture, TEST_POS), 0);
        assert_eq!(open_count_at(&fixture, TEST_POS.east()), 0);
    }

    #[test]
    fn a_spectator_cannot_open_a_double_chest_that_still_has_to_roll_loot() {
        let (fixture, player) = double_chest_fixture("chest_double_loot_spectator");
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS.east())
            .expect("a placed chest has a block entity");
        let mut nbt = NbtCompound::new();
        nbt.insert("LootTable", "minecraft:chests/simple_dungeon");
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("loot NBT decodes");
        block_entity.load_additional(&borrowed);
        player.restore_game_modes(GameType::Spectator, None);

        assert_eq!(
            use_chest_at(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );
        assert!(
            !player.has_container_open(),
            "vanilla `RandomizableContainerBlockEntity.canOpen` keeps spectators out of a \
             chest whose loot has not been generated"
        );
    }
}
