//! Barrel block behavior implementation.
//!
//! Opens a 27-slot container menu when right-clicked.

use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{BlockStateProperties, Direction, EnumProperty};
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, translations};

use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, BlockEntityCreation};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::BLOCK_ENTITIES;
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::BarrelBlockEntity;
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::{ChestKind, chest_with_kind};
use crate::player::Player;
use crate::world::{LevelReader, World};

/// Behavior for barrel blocks.
///
/// Barrels are container block entities with 27 slots (3x9 grid).
/// They use the same menu as chests but cannot form double containers.
#[block_behavior]
pub struct BarrelBlock {
    block: BlockRef,
}

const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING;

impl BarrelBlock {
    /// Creates a new barrel block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehavior for BarrelBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // Barrel faces opposite to the player's look direction (all 6 directions).
        let facing = context.get_nearest_looking_direction().opposite();

        Some(self.block.default_state().set_value(FACING, facing))
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
        // Get the block entity
        let Some(block_entity) = world.get_block_entity(pos) else {
            return InteractionResult::Success;
        };

        let Some(barrel) = block_entity.downcast_ref::<BarrelBlockEntity>() else {
            return InteractionResult::Success;
        };
        let display_name = barrel
            .identity()
            .display_name(|| TextComponent::translated(translations::CONTAINER_BARREL.msg()));
        // `BaseContainerBlockEntity.createMenu` refuses to build a menu for a
        // locked barrel after telling the player which item unlocks it.
        if !barrel.identity().can_open(player) {
            player.close_container();
            ContainerIdentity::send_locked_notification(world, player, pos, display_name);
            player.award_custom_stat(&vanilla_custom_stats::OPEN_BARREL);
            return InteractionResult::Success;
        }

        // `RandomizableContainerBlockEntity.createMenu` rolls the pending loot
        // table before the menu exists, with the opener as loot context.
        barrel.unpack_loot_table(Some(player));

        // Create a container reference from the block entity
        let Some(container_ref) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return InteractionResult::Success;
        };

        // Open the chest menu (3 rows for barrel)
        let inventory = Arc::clone(&player.inventory);
        player.open_menu(display_name, move |context| {
            // `ChestMenu` starts the opener in its constructor, after the
            // previous menu has been closed.
            if let Some(barrel) = block_entity.downcast_ref::<BarrelBlockEntity>() {
                barrel.start_open(context.player);
            }
            let container = container_ref.clone();
            chest_with_kind(
                inventory,
                context.container_id,
                container.clone(),
                3,
                ChestKind::new(container).with_close_hook(move |player| {
                    if let Some(barrel) = block_entity.downcast_ref::<BarrelBlockEntity>() {
                        barrel.stop_open(player);
                    }
                }),
            )
        });

        player.award_custom_stat(&vanilla_custom_stats::OPEN_BARREL);
        // TODO: Anger nearby piglins (PiglinAi.angerNearbyPiglins)

        InteractionResult::Success
    }

    fn tick(&self, _state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        // Vanilla `BarrelBlock.tick` recounts the viewers of an open barrel.
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        if let Some(barrel) = block_entity.downcast_ref::<BarrelBlockEntity>() {
            barrel.recheck_open();
        }
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::BARREL,
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
        // Get the block entity and calculate signal from container contents
        let Some(block_entity) = world.get_block_entity(pos) else {
            return 0;
        };
        // `AbstractContainerMenu.getRedstoneSignalFromContainer` reads every
        // slot, which unpacks a pending loot table first.
        if let Some(barrel) = block_entity.downcast_ref::<BarrelBlockEntity>() {
            barrel.unpack_loot_table(None);
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
    use vecta_registry::sound_events;
    use vecta_registry::vanilla_blocks;
    use vecta_registry::vanilla_items;
    use vecta_utils::types::{GameType, InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::block_entity::{BlockEntity as _, SharedBlockEntity};
    use crate::bootstrap::init_globals;
    use crate::entity::{Entity as _, SharedEntity, next_entity_id};
    use crate::player::ResetReason;
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
        played_sounds, recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);
    /// `BarrelBlockEntity.playSound` uses a fixed volume of 0.5.
    const BARREL_SOUND_VOLUME: f32 = 0.5;
    /// The barrel in `barrel_fixture` keeps the default `FACING=north`, so the
    /// sound leaves the north face: `pos + 0.5 + direction / 2`, encoded at 8x.
    /// That is `(4.5, 64.5, 4.0)`, which no other facing produces.
    const NORTH_FACE_SOUND_POS: IVec3 = IVec3::new(36, 516, 32);

    fn barrel_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::BARREL.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "BarrelUser", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn barrel_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn use_barrel(fixture: &TestWorld, player: &Arc<Player>) -> InteractionResult {
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        BarrelBlock::new(&vanilla_blocks::BARREL).use_without_item(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
            player,
            &barrel_hit(),
            &mut inventory,
        )
    }

    fn lock_barrel(fixture: &TestWorld, lock_item: &str) {
        let block_entity = fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed barrel has a block entity");
        let barrel = block_entity
            .downcast_ref::<BarrelBlockEntity>()
            .expect("the barrel block must create a barrel block entity");
        let mut nbt = NbtCompound::new();
        let mut lock = NbtCompound::new();
        lock.insert("items", lock_item);
        nbt.insert("lock", lock);
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed =
            read_borrowed_compound(&mut Cursor::new(bytes.as_slice())).expect("lock NBT");
        barrel.load_additional(&borrowed);
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

    fn barrel(fixture: &TestWorld) -> SharedBlockEntity {
        fixture
            .world
            .get_block_entity(TEST_POS)
            .expect("a placed barrel has a block entity")
    }

    fn open_count(fixture: &TestWorld) -> i32 {
        barrel(fixture)
            .downcast_ref::<BarrelBlockEntity>()
            .expect("the barrel block must create a barrel block entity")
            .open_count()
    }

    fn is_open(fixture: &TestWorld) -> bool {
        fixture
            .world
            .get_block_state(TEST_POS)
            .get_value(&BlockStateProperties::OPEN)
    }

    /// `BarrelBlockEntity.playSound` reads the block state's `FACING` and offsets
    /// the sound by half a block towards that face.
    fn facing(fixture: &TestWorld) -> Direction {
        fixture
            .world
            .get_block_state(TEST_POS)
            .get_value(&BlockStateProperties::FACING)
    }

    /// A barrel fixture whose viewer receives packets, so the sound the server
    /// plays can be inspected instead of assumed.
    fn listening_barrel_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        assert!(fixture.world.set_block(
            TEST_POS,
            vanilla_blocks::BARREL.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let (connection, packets) = recording_connection();
        let player = TestPlayerBuilder::new(
            Arc::clone(&fixture.world),
            "BarrelListener",
            next_entity_id(),
        )
        .connection(connection)
        .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the barrel's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the sound packet to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    #[test]
    fn opening_a_barrel_counts_the_viewer_and_sets_the_open_property() {
        let (fixture, player) = barrel_fixture("barrel_open_close");
        assert!(!is_open(&fixture));

        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        assert_eq!(open_count(&fixture), 1);
        assert!(
            is_open(&fixture),
            "BarrelBlockEntity.updateBlockState sets OPEN for the first viewer"
        );

        player.close_container();
        assert_eq!(open_count(&fixture), 0);
        assert!(
            !is_open(&fixture),
            "BarrelBlockEntity.updateBlockState clears OPEN for the last viewer"
        );
    }

    #[test]
    fn opening_plays_the_barrel_open_sound_from_the_facing_face() {
        let (fixture, player, packets) = listening_barrel_fixture("barrel_open_sound");

        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);

        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "opening a barrel plays exactly one sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_BARREL_OPEN.key);
        assert_eq!(sounds[0].1, SoundSource::Blocks.as_varint());
        assert_eq!(
            sounds[0].2, NORTH_FACE_SOUND_POS,
            "the sound leaves the face the barrel opens towards"
        );
        assert!((sounds[0].3 - BARREL_SOUND_VOLUME).abs() < f32::EPSILON);
        assert_eq!(facing(&fixture), Direction::North);
    }

    #[test]
    fn closing_plays_the_barrel_close_sound_and_clears_the_open_property() {
        let (fixture, player, packets) = listening_barrel_fixture("barrel_close_sound");
        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        packets.lock().clear();

        player.close_container();

        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1, "closing a barrel plays exactly one sound");
        assert_eq!(sounds[0].0, sound_events::BLOCK_BARREL_CLOSE.key);
        assert_eq!(sounds[0].1, SoundSource::Blocks.as_varint());
        assert_eq!(sounds[0].2, NORTH_FACE_SOUND_POS);
        assert!((sounds[0].3 - BARREL_SOUND_VOLUME).abs() < f32::EPSILON);
        assert!(!is_open(&fixture));
    }

    #[test]
    fn the_first_viewer_of_two_keeps_the_barrel_open() {
        // `ContainerOpenersCounter.incrementOpeners` calls `onOpen` only when the
        // previous count was zero, so the second viewer must not replay the sound.
        let (fixture, player, packets) = listening_barrel_fixture("barrel_two_viewers");
        let second = TestPlayerBuilder::new(Arc::clone(&fixture.world), "BarrelSecond", 2).build();
        second.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));

        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        packets.lock().clear();
        assert_eq!(use_barrel(&fixture, &second), InteractionResult::Success);

        assert_eq!(open_count(&fixture), 2);
        assert!(is_open(&fixture));
        assert!(
            played_sounds(&packets).is_empty(),
            "the second viewer does not re-open the barrel"
        );

        second.close_container();
        assert_eq!(open_count(&fixture), 1);
        assert!(
            is_open(&fixture),
            "one viewer is still there, so the barrel stays open"
        );

        player.close_container();
        assert_eq!(open_count(&fixture), 0);
        assert!(!is_open(&fixture));
    }

    #[test]
    fn a_spectator_does_not_count_as_a_viewer() {
        // `BarrelBlockEntity.startOpen` returns early for spectators.
        let (fixture, player) = barrel_fixture("barrel_spectator");
        player.restore_game_modes(GameType::Spectator, None);

        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        assert_eq!(open_count(&fixture), 0);
        assert!(!is_open(&fixture));
    }

    #[test]
    fn the_block_tick_recounts_the_viewers() {
        let (fixture, player) = barrel_fixture("barrel_recheck_tick");
        register_player(&fixture, &player);
        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        assert_eq!(open_count(&fixture), 1);

        // The menu is still open, but the player walked out of reach, which is
        // exactly the case the scheduled recount exists for.
        player
            .base()
            .try_set_position(DVec3::new(15.5, 64.0, 15.5))
            .expect("the player walks out of range");
        BarrelBlock::new(&vanilla_blocks::BARREL).tick(
            fixture.world.get_block_state(TEST_POS),
            &fixture.world,
            TEST_POS,
        );

        assert_eq!(open_count(&fixture), 0);
        assert!(!is_open(&fixture));
    }

    #[test]
    fn a_locked_barrel_refuses_to_open_without_the_key() {
        let (fixture, player) = barrel_fixture("barrel_locked");
        lock_barrel(&fixture, "minecraft:diamond");

        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        assert!(!player.has_container_open());
    }

    #[test]
    fn a_locked_barrel_opens_with_the_key_in_the_main_hand() {
        let (fixture, player) = barrel_fixture("barrel_locked_key");
        lock_barrel(&fixture, "minecraft:diamond");
        player.inventory.lock().set_item_in_hand(
            InteractionHand::MainHand,
            ItemStack::new(&vanilla_items::DIAMOND),
        );

        assert_eq!(use_barrel(&fixture, &player), InteractionResult::Success);
        assert!(player.has_container_open());
    }
}
