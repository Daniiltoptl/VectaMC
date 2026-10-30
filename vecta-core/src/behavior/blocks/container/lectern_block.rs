//! Lectern block behavior.
//!
//! The lectern either holds a book and opens the lectern menu, or accepts a book
//! from the player's hand. Its `powered` state is a two-tick impulse emitted when
//! a page changes, and the block below is updated so comparators re-read the
//! page-derived signal.

use std::sync::{Arc, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::TaggedRegistryExt as _;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, EnumProperty,
};
use vecta_registry::data_components::vanilla_components::{
    BLOCK_ENTITY_DATA, WRITTEN_BOOK_CONTENT,
};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::vanilla_custom_stats;
use vecta_registry::vanilla_item_tags::ItemTag;
use vecta_registry::{REGISTRY, sound_events, vanilla_block_entity_types, vanilla_blocks};
use vecta_utils::Downcast as _;
use vecta_utils::types::{InteractionHand, UpdateFlags};
use vecta_utils::{BlockPos, BlockStateId, translations};

use crate::{
    behavior::{
        BlockBehavior, InteractionResult, InventoryAccess,
        block::BlockEntityCreation,
        context::{BlockHitResult, BlockPlaceContext},
    },
    block_entity::{BLOCK_ENTITIES, SharedBlockEntity, entities::LecternBlockEntity},
    command::{
        execution::{CommandSource, CommandTextResolver},
        sender::CommandSender,
    },
    inventory::{lock::ContainerRef, menu::kinds::lectern},
    player::Player,
    world::{LevelReader, SignalQueryContext, World},
};

/// Behavior for lectern blocks.
#[block_behavior]
pub struct LecternBlock {
    block: BlockRef,
}

const FACING: &EnumProperty<Direction> = &BlockStateProperties::HORIZONTAL_FACING;
const POWERED: &BoolProperty = &BlockStateProperties::POWERED;
const HAS_BOOK: &BoolProperty = &BlockStateProperties::HAS_BOOK;

impl LecternBlock {
    /// Creates a new lectern block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Mirrors `LecternBlock.tryPlaceBook`: returns whether the interaction was
    /// accepted, regardless of whether a block entity could take the book.
    fn try_place_book(
        world: &Arc<World>,
        pos: BlockPos,
        state: BlockStateId,
        player: &Player,
        inv: &InventoryAccess,
    ) -> bool {
        if state.get_value(HAS_BOOK) {
            return false;
        }
        // Snapshot the stack without the inventory lock: resolving book pages can
        // read the inventory and the world, and must not run under that lock.
        let placed = inv.with_item(|stack| stack.copy_with_count(1));
        let placed = Self::resolve_book(placed, player);

        if let Some(block_entity) = world.get_block_entity(pos)
            && let Some(lectern) = block_entity.downcast_ref::<LecternBlockEntity>()
        {
            lectern.set_book(placed);
            world.play_block_sound(&sound_events::ITEM_BOOK_PUT, pos, 1.0, 1.0, None);
        }

        let has_infinite_materials = player.has_infinite_materials();
        inv.with_item(|stack| stack.consume(1, has_infinite_materials));
        true
    }

    /// Resolves a written book's pages before it becomes readable on the lectern.
    ///
    /// Block behaviors only see `&Player`, so the resolution source is the
    /// console fallback with this player as the entity override. That is the same
    /// snapshot-based path the written book item uses, and it never runs while an
    /// inventory lock is held.
    fn resolve_book(stack: ItemStack, player: &Player) -> ItemStack {
        let Some(content) = stack
            .get(WRITTEN_BOOK_CONTENT)
            .filter(|content| !content.resolved())
            .cloned()
        else {
            return stack;
        };
        let Some(server) = player.server.upgrade() else {
            return stack;
        };

        let source = CommandSource::new(CommandSender::Console, server);
        let resolver = CommandTextResolver::with_entity_override(&source, player);
        let resolved_content = content
            .resolve(&resolver)
            .unwrap_or_else(|| content.mark_resolved());

        let mut resolved_stack = stack;
        resolved_stack.set(WRITTEN_BOOK_CONTENT, resolved_content);
        resolved_stack
    }

    fn open_screen(world: &Arc<World>, pos: BlockPos, player: &Player) {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        let Some(container) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return;
        };
        let menu_block_entity: SharedBlockEntity = block_entity;
        player.open_menu(
            TextComponent::translated(translations::CONTAINER_LECTERN.msg()),
            move |context| lectern(context.container_id, container, menu_block_entity),
        );
        player.award_custom_stat(&vanilla_custom_stats::INTERACT_WITH_LECTERN);
    }
}

impl BlockBehavior for LecternBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // Vanilla only pre-fills `has_book` for creative placements of a lectern
        // item that carries stored block entity data with a book in it.
        let has_book = context
            .player()
            .is_some_and(|player| player.abilities.lock().instabuild)
            && context.with_item(|stack| {
                stack
                    .get(BLOCK_ENTITY_DATA)
                    .is_some_and(|data| data.data().as_compound().get("Book").is_some())
            });

        Some(
            self.block
                .default_state()
                .set_value(FACING, context.horizontal_direction().opposite())
                .set_value(HAS_BOOK, has_book),
        )
    }

    fn use_item_on(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        hand: InteractionHand,
        _hit_result: &BlockHitResult,
        inv: &mut InventoryAccess,
    ) -> InteractionResult {
        if state.get_value(HAS_BOOK) {
            return InteractionResult::TryEmptyHandInteraction;
        }

        let is_lectern_book = inv.with_item(|stack| {
            !stack.is_empty()
                && REGISTRY
                    .items
                    .is_in_tag(stack.item(), &ItemTag::LECTERN_BOOKS)
        });
        if is_lectern_book {
            return if Self::try_place_book(world, pos, state, player, inv) {
                InteractionResult::Success
            } else {
                InteractionResult::Pass
            };
        }

        let empty_main_hand =
            inv.with_item(|stack| stack.is_empty()) && hand == InteractionHand::MainHand;
        if empty_main_hand {
            InteractionResult::Pass
        } else {
            InteractionResult::TryEmptyHandInteraction
        }
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
        if state.get_value(HAS_BOOK) {
            Self::open_screen(world, pos, player);
            InteractionResult::Success
        } else {
            InteractionResult::Consume
        }
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::LECTERN,
            level,
            pos,
            state,
        ))
    }

    /// The powered impulse expires two ticks after the page changed.
    fn tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        if state.get_value(POWERED) {
            let updated_state = state.set_value(POWERED, false);
            world.set_block(pos, updated_state, UpdateFlags::UPDATE_ALL);
            world.update_neighbors_at(pos.below(), &vanilla_blocks::LECTERN);
        }
    }

    fn affect_neighbors_after_removal(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _moved_by_piston: bool,
    ) {
        if state.get_value(POWERED) {
            world.update_neighbors_at(pos.below(), &vanilla_blocks::LECTERN);
        }
    }

    fn is_signal_source(&self, _state: BlockStateId, _context: SignalQueryContext) -> bool {
        true
    }

    /// Vanilla `LecternBlock.ownSignal`.
    fn get_own_signal(
        &self,
        state: BlockStateId,
        _world: &dyn LevelReader,
        _pos: BlockPos,
        _context: SignalQueryContext,
    ) -> i32 {
        if state.get_value(POWERED) { 15 } else { 0 }
    }

    /// Vanilla `LecternBlock.getDirectSignal`: only upwards, only while powered.
    fn get_direct_signal(
        &self,
        state: BlockStateId,
        _world: &dyn LevelReader,
        _pos: BlockPos,
        direction: Direction,
        _context: SignalQueryContext,
    ) -> i32 {
        if direction == Direction::Up && state.get_value(POWERED) {
            15
        } else {
            0
        }
    }

    fn has_analog_output_signal(&self, _state: BlockStateId) -> bool {
        true
    }

    /// Vanilla `LecternBlock.getAnalogOutputSignal`: the page-derived signal of
    /// the stored book, zero without a book.
    fn get_analog_output_signal(
        &self,
        state: BlockStateId,
        world: &dyn LevelReader,
        pos: BlockPos,
        _direction: Direction,
    ) -> i32 {
        if !state.get_value(HAS_BOOK) {
            return 0;
        }
        world.get_block_entity(pos).map_or(0, |block_entity| {
            block_entity
                .downcast_ref::<LecternBlockEntity>()
                .map_or(0, LecternBlockEntity::redstone_signal)
        })
    }
}
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use glam::DVec3;
    use vecta_registry::data_components::components::{Filterable, WritableBookContent};
    use vecta_registry::data_components::vanilla_components::WRITABLE_BOOK_CONTENT;
    use vecta_registry::{vanilla_entities, vanilla_items};
    use vecta_utils::{ChunkPos, WorldAabb};

    use super::*;
    use crate::bootstrap::init_globals;
    use crate::entity::Entity as _;
    use crate::entity::entities::ItemEntity;
    use crate::test_support::{
        TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk,
    };

    const TEST_POS: BlockPos = BlockPos::new(6, 64, 6);
    const DROP_SEARCH_MARGIN: f64 = 1.0;

    fn test_state() -> BlockStateId {
        vanilla_blocks::LECTERN.default_state()
    }

    fn test_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(
                f64::from(TEST_POS.x()) + 0.5,
                f64::from(TEST_POS.y()) + 1.0,
                f64::from(TEST_POS.z()) + 0.5,
            ),
            direction: Direction::Up,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn writable_book(pages: usize) -> ItemStack {
        let mut book = ItemStack::new(&vanilla_items::WRITABLE_BOOK);
        book.set(
            WRITABLE_BOOK_CONTENT,
            WritableBookContent::new(
                (0..pages)
                    .map(|page| Filterable::pass_through(format!("page {page}")))
                    .collect(),
            )
            .expect("test pages are valid"),
        );
        book
    }

    fn setup(key: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let world_fixture = fresh_test_world(key);
        let world = &world_fixture.world;
        insert_ready_full_chunk(world, ChunkPos::from_block_pos(TEST_POS));
        assert!(world.set_block(TEST_POS, test_state(), UpdateFlags::UPDATE_ALL));
        let player = TestPlayerBuilder::new(Arc::clone(world), "LecternTester", 1).build();
        player.base().set_position_local(DVec3::new(6.5, 64.0, 6.5));
        (world_fixture, player)
    }

    /// Runs `f` with the lectern block entity while keeping the entity alive.
    fn with_lectern<R>(world: &Arc<World>, f: impl FnOnce(&LecternBlockEntity) -> R) -> R {
        let block_entity = world
            .get_block_entity(TEST_POS)
            .expect("the lectern block creates a block entity");
        let lectern = block_entity
            .downcast_ref::<LecternBlockEntity>()
            .expect("registered lectern uses its concrete block entity");
        f(lectern)
    }

    fn has_book_state(world: &Arc<World>) -> bool {
        world
            .get_block_state(TEST_POS)
            .get_value(&BlockStateProperties::HAS_BOOK)
    }

    #[test]
    fn placing_a_book_consumes_one_item_and_sets_has_book() {
        let (world_fixture, player) = setup("lectern_place_book");
        let world = &world_fixture.world;
        let behavior = LecternBlock::new(&vanilla_blocks::LECTERN);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        player
            .inventory
            .lock()
            .set_selected_item(ItemStack::with_count(&vanilla_items::WRITABLE_BOOK, 2));

        assert_eq!(
            behavior.use_item_on(
                test_state(),
                world,
                TEST_POS,
                &player,
                InteractionHand::MainHand,
                &test_hit(),
                &mut inventory,
            ),
            InteractionResult::Success,
        );

        assert_eq!(player.inventory.lock().get_selected_item().count(), 1);
        assert!(with_lectern(world, LecternBlockEntity::has_book));
        assert_eq!(with_lectern(world, |lectern| lectern.book().count()), 1);
        assert!(has_book_state(world), "the block state follows the book");
    }

    #[test]
    fn a_lectern_with_a_book_falls_back_to_the_empty_hand() {
        let (world_fixture, player) = setup("lectern_already_full");
        let world = &world_fixture.world;
        with_lectern(world, |lectern| lectern.set_book(writable_book(1)));
        let behavior = LecternBlock::new(&vanilla_blocks::LECTERN);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        player
            .inventory
            .lock()
            .set_selected_item(ItemStack::new(&vanilla_items::WRITABLE_BOOK));

        assert_eq!(
            behavior.use_item_on(
                world.get_block_state(TEST_POS),
                world,
                TEST_POS,
                &player,
                InteractionHand::MainHand,
                &test_hit(),
                &mut inventory,
            ),
            InteractionResult::TryEmptyHandInteraction,
        );
        assert_eq!(player.inventory.lock().get_selected_item().count(), 1);
    }

    #[test]
    fn an_empty_lectern_consumes_the_interaction() {
        let (world_fixture, player) = setup("lectern_empty_interaction");
        let world = &world_fixture.world;
        let behavior = LecternBlock::new(&vanilla_blocks::LECTERN);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);

        assert_eq!(
            behavior.use_without_item(
                test_state(),
                world,
                TEST_POS,
                &player,
                &test_hit(),
                &mut inventory,
            ),
            InteractionResult::Consume,
        );
    }

    #[test]
    fn the_comparator_follows_the_page_and_needs_a_book() {
        let (world_fixture, _player) = setup("lectern_comparator");
        let world = &world_fixture.world;
        let behavior = LecternBlock::new(&vanilla_blocks::LECTERN);

        assert_eq!(
            behavior.get_analog_output_signal(test_state(), world, TEST_POS, Direction::North),
            0,
            "a book-less lectern reports nothing"
        );

        with_lectern(world, |lectern| lectern.set_book(writable_book(4)));
        let state = world.get_block_state(TEST_POS);
        assert_eq!(
            behavior.get_analog_output_signal(state, world, TEST_POS, Direction::North),
            1,
        );

        with_lectern(world, |lectern| lectern.set_page(3));
        assert_eq!(
            behavior.get_analog_output_signal(state, world, TEST_POS, Direction::North),
            15,
        );
    }

    #[test]
    fn breaking_the_lectern_drops_the_book_in_front_of_it() {
        let (world_fixture, _player) = setup("lectern_break");
        let world = &world_fixture.world;
        with_lectern(world, |lectern| lectern.set_book(writable_book(2)));
        assert!(has_book_state(world));

        assert!(world.set_block(
            TEST_POS,
            vanilla_blocks::AIR.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));

        let block_min = DVec3::new(
            f64::from(TEST_POS.x()),
            f64::from(TEST_POS.y()),
            f64::from(TEST_POS.z()),
        );
        let search =
            WorldAabb::from_min_max(block_min, block_min + DVec3::ONE).inflate(DROP_SEARCH_MARGIN);
        let drops = world.get_entities_in_aabb_matching(&search, |entity| {
            entity.entity_type() == &vanilla_entities::ITEM
        });
        assert_eq!(drops.len(), 1, "the lectern drops exactly one stack");
        let dropped = drops[0]
            .downcast_ref::<ItemEntity>()
            .map(ItemEntity::get_item)
            .expect("the dropped entity is an item");
        assert_eq!(dropped.item(), &*vanilla_items::WRITABLE_BOOK);
        assert_eq!(dropped.count(), 1);
    }

    #[test]
    fn placement_starts_without_a_book() {
        let (world_fixture, player) = setup("lectern_placement");
        let world = &world_fixture.world;
        let behavior = LecternBlock::new(&vanilla_blocks::LECTERN);
        let inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        let context = BlockPlaceContext::new(
            world,
            crate::behavior::context::PlacementSource::player_hand(&player, &inventory),
            &test_hit(),
        );

        let state = behavior
            .get_state_for_placement(&context)
            .expect("the lectern always has a placement state");
        assert!(!state.get_value(&BlockStateProperties::HAS_BOOK));
        assert!(
            state.get_value(FACING).is_horizontal(),
            "the lectern is a horizontal directional block"
        );
    }
}
