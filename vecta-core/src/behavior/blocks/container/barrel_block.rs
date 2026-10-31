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
use crate::inventory::menu::kinds::chest;
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
            chest(inventory, context.container_id, container_ref, 3)
        });

        player.award_custom_stat(&vanilla_custom_stats::OPEN_BARREL);
        // TODO: Anger nearby piglins (PiglinAi.angerNearbyPiglins)
        // TODO: Implement ContainerOpenersCounter to track open state, play sounds,
        //       and update OPEN block property. Requires scheduled block ticks (scheduleTick)
        //       for recheck functionality. See vanilla BarrelBlockEntity and ContainerOpenersCounter.

        InteractionResult::Success
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

    use glam::DVec3;
    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use simdnbt::owned::NbtCompound;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::vanilla_blocks;
    use vecta_registry::vanilla_items;
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
