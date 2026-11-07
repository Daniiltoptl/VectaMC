//! The composter: fill levels 0–8, bone meal output, and automation views.
//!
//! Vanilla `ComposterBlock` has no block entity: its hoppers-facing
//! `InputContainer`/`OutputContainer` are transient `SimpleContainer`s that
//! perform their world mutation from `setChanged`. Vecta mirrors that with the
//! `WorldlyContainerHolder` capability, so `container_at` consumers (hopper,
//! dropper, crafter) see the same views without a special case.

use std::sync::Arc;

use glam::DVec3;
use vecta_macros::block_behavior;
use vecta_registry::blocks::{
    BlockRef,
    block_state_ext::BlockStateExt as _,
    properties::{BlockStateProperties, Direction, IntProperty},
};
use vecta_registry::data_components::vanilla_components::{COMPOSTABLE, Compostable};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::loot_table::{LootContext, loot_context_params};
use vecta_registry::stat::vanilla_stat_types;
use vecta_registry::{
    REGISTRY, level_events, sound_events, vanilla_blocks, vanilla_entities, vanilla_game_events,
    vanilla_items,
};
use vecta_utils::locks::SyncMutex;
use vecta_utils::random::Random as _;
use vecta_utils::types::{InteractionHand, UpdateFlags};
use vecta_utils::{BlockPos, BlockStateId, DowncastType, DowncastTypeKey};

use crate::{
    behavior::{
        BlockBehavior, BlockHitResult, BlockPlaceContext, InteractionResult, InventoryAccess,
        worldly_container_holder::WorldlyContainerHolder,
    },
    entity::{
        Entity, ai::path::PathComputationType, entities::ItemEntity, entity_loot_ref,
        loot_equipment, next_entity_id,
    },
    inventory::container::Container,
    inventory::prelude::ContainerRef,
    player::Player,
    world::{LevelReader, World, game_event::GameEventContext},
};

/// Vanilla composter behavior required by analog signal readers.
#[block_behavior]
pub struct ComposterBlock {
    block: BlockRef,
}

const LEVEL_COMPOSTER: &IntProperty = &BlockStateProperties::LEVEL_COMPOSTER;
/// Vanilla `ComposterBlock.READY`.
const READY: u8 = 8;
/// Vanilla `ComposterBlock.MAX_LEVEL`.
const MAX_LEVEL: u8 = 7;
/// Vanilla `ComposterBlock` schedules its ready tick 20 game ticks ahead.
const READY_DELAY: i32 = 20;

impl ComposterBlock {
    /// Creates composter behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// The composter fill level of `state` (`LEVEL`, 0–8).
    fn fill_level(state: BlockStateId) -> u8 {
        state.get_value(LEVEL_COMPOSTER)
    }

    /// Evaluates a compostable item's layer provider against the
    /// `BLOCK_INTERACT` context (`ComposterBlock.addLayer`).
    fn layers_to_add(
        compostable: &Compostable,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        source: Option<&dyn Entity>,
    ) -> i32 {
        let Some(provider) = REGISTRY.context_int_providers.by_key(&compostable.layers) else {
            return 0;
        };
        let center = (
            f64::from(pos.x()) + 0.5,
            f64::from(pos.y()) + 0.5,
            f64::from(pos.z()) + 0.5,
        );
        let interacting_equipment = source.and_then(loot_equipment);
        world.random().with_level(|rng| {
            let mut context = LootContext::new(rng)
                .with_data(world.as_ref())
                .with_parameter(loot_context_params::BLOCK_STATE, state)
                .with_parameter(loot_context_params::ORIGIN, center);
            if let Some(source) = source {
                context = context.with_parameter(
                    loot_context_params::INTERACTING_ENTITY,
                    entity_loot_ref(source, interacting_equipment.as_ref()),
                );
            }
            provider.get_int(&mut context)
        })
    }

    /// Vanilla `ComposterBlock.addLayer`: adds the item's layers and schedules
    /// the ready tick when the composter fills up.
    fn add_layer(
        source: Option<&dyn Entity>,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        compostable: &Compostable,
    ) -> BlockStateId {
        let layers = Self::layers_to_add(compostable, state, world, pos, source);
        if layers <= 0 {
            return state;
        }
        let new_level =
            (i32::from(Self::fill_level(state)) + layers).clamp(0, i32::from(MAX_LEVEL));
        let new_level = u8::try_from(new_level).unwrap_or(MAX_LEVEL);
        let new_state = state.set_value(LEVEL_COMPOSTER, new_level);
        world.set_block(pos, new_state, UpdateFlags::UPDATE_ALL);
        world.game_event(
            &vanilla_game_events::BLOCK_CHANGE,
            pos,
            &GameEventContext::new(source, Some(new_state)),
        );
        if new_level == MAX_LEVEL {
            world.schedule_block_tick_default(pos, &vanilla_blocks::COMPOSTER, READY_DELAY);
        }
        new_state
    }

    /// Vanilla `ComposterBlock.empty`: resets to level 0.
    fn empty(
        source: Option<&dyn Entity>,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
    ) -> BlockStateId {
        let new_state = state.set_value(LEVEL_COMPOSTER, 0);
        world.set_block(pos, new_state, UpdateFlags::UPDATE_ALL);
        world.game_event(
            &vanilla_game_events::BLOCK_CHANGE,
            pos,
            &GameEventContext::new(source, Some(new_state)),
        );
        new_state
    }

    /// Vanilla `ComposterBlock.extractProduce`: ejects one bone meal above the
    /// composter and empties it.
    fn extract_produce(
        source: Option<&dyn Entity>,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
    ) {
        // `offsetRandomXZ` draws from the level random. The initial velocity
        // comes from the new entity's own unseeded random, so it must not
        // advance the level stream.
        let position = world.random().with_level(|rng| {
            DVec3::new(
                f64::from(pos.x()) + 0.5 + f64::from((rng.next_f32() - 0.5) * 0.7),
                f64::from(pos.y()) + 1.01,
                f64::from(pos.z()) + 0.5 + f64::from((rng.next_f32() - 0.5) * 0.7),
            )
        });
        let velocity = DVec3::new(
            rand::random::<f64>() * 0.2 - 0.1,
            0.2,
            rand::random::<f64>() * 0.2 - 0.1,
        );
        let entity = Arc::new(ItemEntity::with_item_and_velocity(
            &vanilla_entities::ITEM,
            next_entity_id(),
            position,
            ItemStack::new(&vanilla_items::BONE_MEAL),
            velocity,
            Arc::downgrade(world),
        ));
        entity.set_default_pickup_delay();
        if let Err(error) = world.try_add_entity(entity) {
            log::warn!("Failed to spawn the composter produce: {error}");
        }

        Self::empty(source, state, world, pos);
        world.play_block_sound(&sound_events::BLOCK_COMPOSTER_EMPTY, pos, 1.0, 1.0, None);
    }
}

impl BlockBehavior for ComposterBlock {
    fn as_worldly_container_holder(&self) -> Option<&dyn WorldlyContainerHolder> {
        Some(self)
    }

    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.block.default_state())
    }

    /// Vanilla `ComposterBlock.onPlace`: a composter placed at level 7 (piston
    /// move) still ripens.
    fn on_place(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _old_state: BlockStateId,
        _moved_by_piston: bool,
    ) {
        if Self::fill_level(state) == MAX_LEVEL {
            world.schedule_block_tick_default(pos, self.block, READY_DELAY);
        }
    }

    fn tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        if Self::fill_level(state) == MAX_LEVEL {
            let new_state = state.set_value(LEVEL_COMPOSTER, READY);
            world.set_block(pos, new_state, UpdateFlags::UPDATE_ALL);
            world.play_block_sound(&sound_events::BLOCK_COMPOSTER_READY, pos, 1.0, 1.0, None);
        }
    }

    fn use_item_on(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hand: InteractionHand,
        _hit_result: &BlockHitResult,
        inv: &mut InventoryAccess,
    ) -> InteractionResult {
        let fill_level = Self::fill_level(state);
        let compostable = inv.with_item(|stack| stack.get(COMPOSTABLE).cloned());
        if fill_level < READY
            && let Some(compostable) = compostable
        {
            if fill_level < MAX_LEVEL {
                let new_state = Self::add_layer(Some(player), state, world, pos, &compostable);
                world.level_event(
                    level_events::COMPOSTER_FILL,
                    pos,
                    i32::from(state != new_state),
                    None,
                );
                inv.with_item(|stack| {
                    player.award_stat(&vanilla_stat_types::ITEM_USED, stack.item());
                });
                inv.with_item(|stack| {
                    stack.consume_one(player.has_infinite_materials());
                });
            }
            return InteractionResult::Success;
        }
        InteractionResult::TryEmptyHandInteraction
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
        if Self::fill_level(state) == READY {
            Self::extract_produce(Some(player), state, world, pos);
            return InteractionResult::Success;
        }
        InteractionResult::Pass
    }

    fn has_analog_output_signal(&self, _state: BlockStateId) -> bool {
        true
    }

    fn get_analog_output_signal(
        &self,
        state: BlockStateId,
        _world: &dyn LevelReader,
        _pos: BlockPos,
        _direction: Direction,
    ) -> i32 {
        i32::from(Self::fill_level(state))
    }

    fn is_pathfindable(
        &self,
        _state: BlockStateId,
        _computation_type: PathComputationType,
    ) -> bool {
        false
    }
}

impl WorldlyContainerHolder for ComposterBlock {
    /// Vanilla `ComposterBlock.getContainer`: the output view when ready, the
    /// input view below level 7, and an empty view at level 7.
    fn get_container(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
    ) -> ContainerRef {
        match Self::fill_level(state) {
            READY => ContainerRef::from(Arc::new(SyncMutex::new(OutputContainer::new(
                state,
                Arc::clone(world),
                pos,
            )))),
            level if level < MAX_LEVEL => ContainerRef::from(Arc::new(SyncMutex::new(
                InputContainer::new(state, Arc::clone(world), pos),
            ))),
            _ => ContainerRef::from(Arc::new(SyncMutex::new(EmptyContainer))),
        }
    }
}

/// Vanilla `ComposterBlock.InputContainer`: hoppers above feed one compostable
/// item, which `set_changed` immediately composts.
struct InputContainer {
    state: BlockStateId,
    world: Arc<World>,
    pos: BlockPos,
    changed: bool,
    items: [ItemStack; 1],
}

// SAFETY: This Vecta-owned key uniquely identifies the composter's input view.
unsafe impl DowncastType for InputContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/composter_input");
}

impl InputContainer {
    fn new(state: BlockStateId, world: Arc<World>, pos: BlockPos) -> Self {
        Self {
            state,
            world,
            pos,
            changed: false,
            items: [ItemStack::empty()],
        }
    }
}

impl Container for InputContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_max_stack_size(&self) -> i32 {
        1
    }

    /// `SimpleContainer.setItem` calls `setChanged`, which composts at once.
    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        let max_size = self.get_max_stack_size_for_item(&stack);
        if stack.count() > max_size {
            stack.set_count(max_size);
        }
        self.items[slot] = stack;
        self.set_changed();
    }

    fn slots_for_face(&self, direction: Direction) -> Option<&'static [usize]> {
        if direction == Direction::Up {
            Some(&[0])
        } else {
            Some(&[])
        }
    }

    fn can_place_item_through_face(
        &self,
        _slot: usize,
        stack: &ItemStack,
        direction: Direction,
    ) -> bool {
        !self.changed && direction == Direction::Up && stack.get(COMPOSTABLE).is_some()
    }

    fn can_take_item_through_face(
        &self,
        _slot: usize,
        _stack: &ItemStack,
        _direction: Direction,
    ) -> bool {
        false
    }

    fn set_changed(&mut self) {
        let contents = self.get_item(0).clone();
        if contents.is_empty() {
            return;
        }
        let Some(compostable) = contents.get(COMPOSTABLE) else {
            return;
        };
        self.changed = true;
        let new_state =
            ComposterBlock::add_layer(None, self.state, &self.world, self.pos, compostable);
        self.world.level_event(
            level_events::COMPOSTER_FILL,
            self.pos,
            i32::from(self.state != new_state),
            None,
        );
        self.remove_item_no_update(0);
    }
}

/// Vanilla `ComposterBlock.OutputContainer`: hoppers below take the ready bone
/// meal, which `set_changed` empties the composter.
struct OutputContainer {
    state: BlockStateId,
    world: Arc<World>,
    pos: BlockPos,
    changed: bool,
    items: [ItemStack; 1],
}

// SAFETY: This Vecta-owned key uniquely identifies the composter's output view.
unsafe impl DowncastType for OutputContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/composter_output");
}

impl OutputContainer {
    fn new(state: BlockStateId, world: Arc<World>, pos: BlockPos) -> Self {
        Self {
            state,
            world,
            pos,
            changed: false,
            items: [ItemStack::new(&vanilla_items::BONE_MEAL)],
        }
    }
}

impl Container for OutputContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_max_stack_size(&self) -> i32 {
        1
    }

    /// `SimpleContainer.removeItem` calls `setChanged` before the hopper knows
    /// whether the bone meal fits, so a failed pull still empties the
    /// composter and the bone meal is lost, as in vanilla.
    fn remove_item(&mut self, slot: usize, count: i32) -> ItemStack {
        let item = &mut self.items[slot];
        if item.is_empty() || count <= 0 {
            return ItemStack::empty();
        }
        let removed = item.split(count);
        self.set_changed();
        removed
    }

    /// `SimpleContainer.setItem` calls `setChanged`.
    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        let max_size = self.get_max_stack_size_for_item(&stack);
        if stack.count() > max_size {
            stack.set_count(max_size);
        }
        self.items[slot] = stack;
        self.set_changed();
    }

    fn slots_for_face(&self, direction: Direction) -> Option<&'static [usize]> {
        if direction == Direction::Down {
            Some(&[0])
        } else {
            Some(&[])
        }
    }

    fn can_place_item_through_face(
        &self,
        _slot: usize,
        _stack: &ItemStack,
        _direction: Direction,
    ) -> bool {
        false
    }

    fn can_take_item_through_face(
        &self,
        _slot: usize,
        stack: &ItemStack,
        direction: Direction,
    ) -> bool {
        !self.changed && direction == Direction::Down && stack.is(&vanilla_items::BONE_MEAL)
    }

    fn set_changed(&mut self) {
        ComposterBlock::empty(None, self.state, &self.world, self.pos);
        self.changed = true;
    }
}

/// Vanilla `ComposterBlock.EmptyContainer`: a level-7 composter offers no slots
/// but still counts as a container, so a dropper facing it keeps its item
/// instead of ejecting it.
struct EmptyContainer;

// SAFETY: This Vecta-owned key uniquely identifies the composter's empty view.
unsafe impl DowncastType for EmptyContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/composter_empty");
}

impl Container for EmptyContainer {
    fn items(&self) -> &[ItemStack] {
        &[]
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut []
    }

    fn slots_for_face(&self, _direction: Direction) -> Option<&'static [usize]> {
        Some(&[])
    }

    fn can_place_item_through_face(
        &self,
        _slot: usize,
        _stack: &ItemStack,
        _direction: Direction,
    ) -> bool {
        false
    }

    fn can_take_item_through_face(
        &self,
        _slot: usize,
        _stack: &ItemStack,
        _direction: Direction,
    ) -> bool {
        false
    }

    fn set_changed(&mut self) {}
}

#[cfg(test)]
mod tests {
    use std::slice;
    use std::sync::Arc;

    use glam::{DVec3, IVec3};
    use vecta_registry::blocks::properties::BlockStateProperties;
    use vecta_registry::blocks::properties::Direction;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::sound_events;
    use vecta_registry::vanilla_blocks;
    use vecta_registry::vanilla_items;
    use vecta_utils::Downcast as _;
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos};

    use super::*;
    use crate::behavior::BLOCK_BEHAVIORS;
    use crate::behavior::InventoryAccess;
    use crate::behavior::blocks::redstone::DropperBlock;
    use crate::bootstrap::init_globals;
    use crate::entity::next_entity_id;
    use crate::inventory::lock::ContainerLockGuard;
    use crate::player::ResetReason;
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, advance_test_game_time_to, dropped_items,
        fresh_test_world, insert_ready_full_chunk, level_events, played_sounds,
        recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);
    const ABOVE: BlockPos = BlockPos::new(4, 65, 4);
    /// Server sounds leave the block centre, encoded at 8x: `(4, 64, 4)`.
    const BLOCK_CENTRE_SOUND_POS: IVec3 = IVec3::new(36, 516, 36);
    /// Vanilla event id for the composter fill feedback.
    const COMPOSTER_FILL_EVENT: i32 = level_events::COMPOSTER_FILL;

    fn composter_state(fixture: &TestWorld, pos: BlockPos) -> BlockStateId {
        fixture.world.get_block_state(pos)
    }

    fn composter_level(fixture: &TestWorld, pos: BlockPos) -> u8 {
        composter_state(fixture, pos).get_value(LEVEL_COMPOSTER)
    }

    fn place_composter(fixture: &TestWorld, pos: BlockPos, level: u8) {
        let state = vanilla_blocks::COMPOSTER
            .default_state()
            .set_value(LEVEL_COMPOSTER, level);
        assert!(fixture.world.set_block(pos, state, UpdateFlags::UPDATE_ALL));
    }

    fn composter_fixture(name: &'static str) -> (TestWorld, Arc<Player>) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place_composter(&fixture, TEST_POS, 0);
        let player = TestPlayerBuilder::new(Arc::clone(&fixture.world), "Composter", 1).build();
        player.base().set_position_local(DVec3::new(4.5, 64.0, 4.5));
        (fixture, player)
    }

    fn listening_composter_fixture(name: &'static str) -> (TestWorld, Arc<Player>, SentPackets) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place_composter(&fixture, TEST_POS, 0);
        let (connection, packets) = recording_connection();
        let player =
            TestPlayerBuilder::new(Arc::clone(&fixture.world), "CompListener", next_entity_id())
                .connection(connection)
                .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the composter's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for level events to reach it"
        );
        packets.lock().clear();
        (fixture, player, packets)
    }

    fn composter_hit() -> BlockHitResult {
        BlockHitResult {
            location: DVec3::new(4.5, 64.5, 5.0),
            direction: Direction::South,
            block_pos: TEST_POS,
            miss: false,
            inside: false,
            world_border_hit: false,
        }
    }

    fn click(fixture: &TestWorld, player: &Arc<Player>, pos: BlockPos) -> InteractionResult {
        let behavior = ComposterBlock::new(&vanilla_blocks::COMPOSTER);
        let state = composter_state(fixture, pos);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        let result = behavior.use_item_on(
            state,
            &fixture.world,
            pos,
            player,
            InteractionHand::MainHand,
            &composter_hit(),
            &mut inventory,
        );
        // Vanilla `ServerPlayerGameMode.useItemOn`: `TRY_WITH_EMPTY_HAND` on the
        // main hand immediately falls through to the empty-hand interaction.
        if result == InteractionResult::TryEmptyHandInteraction {
            return behavior.use_without_item(
                state,
                &fixture.world,
                pos,
                player,
                &composter_hit(),
                &mut inventory,
            );
        }
        result
    }

    fn hand_item(player: &Arc<Player>) -> ItemStack {
        player
            .inventory
            .lock()
            .get_item_in_hand(InteractionHand::MainHand)
            .clone()
    }

    fn give_in_hand(player: &Arc<Player>, item: &ItemStack) {
        player
            .inventory
            .lock()
            .set_item_in_hand(InteractionHand::MainHand, item.clone());
    }

    fn tick_block_entities(fixture: &TestWorld, ticks: usize) {
        for _ in 0..ticks {
            advance_test_game_time_to(&fixture.world, fixture.world.game_time() + 1);
            fixture
                .world
                .block_entity_tickers()
                .tick(&fixture.world, true);
        }
    }

    fn container_ref(fixture: &TestWorld, pos: BlockPos) -> ContainerRef {
        let block_entity = fixture
            .world
            .get_block_entity(pos)
            .unwrap_or_else(|| panic!("{pos:?} should hold a block entity"));
        ContainerRef::from_block_entity(block_entity)
            .unwrap_or_else(|| panic!("the block entity at {pos:?} should be a container"))
    }

    fn hopper_slot(fixture: &TestWorld, pos: BlockPos, slot: usize) -> ItemStack {
        let container = container_ref(fixture, pos);
        let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        guard
            .get(container.container_id())
            .map_or_else(ItemStack::empty, |locked| locked.get_item(slot).clone())
    }

    fn set_hopper_slot(fixture: &TestWorld, pos: BlockPos, slot: usize, stack: ItemStack) {
        let container = container_ref(fixture, pos);
        let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        assert!(guard.set_item(container.container_id(), slot, stack));
    }

    /// A hopper above the composter at `TEST_POS`, facing down into it.
    fn hopper_fixture(name: &'static str) -> TestWorld {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        place_composter(&fixture, TEST_POS, 0);
        let hopper = vanilla_blocks::HOPPER
            .default_state()
            .set_value(&BlockStateProperties::FACING_HOPPER, Direction::Down);
        assert!(
            fixture
                .world
                .set_block(ABOVE, hopper, UpdateFlags::UPDATE_ALL)
        );
        fixture
    }

    /// Vanilla `ComposterBlock.addLayer` succeeds unconditionally at level 0:
    /// the configured `compostable/*` dispatchers check `level == 0` first.
    #[test]
    fn a_click_at_level_zero_always_adds_a_layer() {
        let (fixture, player, packets) = listening_composter_fixture("composter_click_level_zero");
        give_in_hand(&player, &ItemStack::new(&vanilla_items::OAK_SAPLING));

        assert_eq!(
            click(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );

        assert_eq!(composter_level(&fixture, TEST_POS), 1);
        assert!(
            hand_item(&player).is_empty(),
            "vanilla consumes the item even when no layer is added"
        );
        let events: Vec<_> = level_events(&packets)
            .into_iter()
            .filter(|(event, _, _)| *event == COMPOSTER_FILL_EVENT)
            .collect();
        assert_eq!(events.len(), 1, "one fill feedback event per click");
        assert_eq!(events[0].1, TEST_POS);
        assert_eq!(
            events[0].2, 1,
            "a layer was added, so the client plays success"
        );
    }

    #[test]
    fn a_non_compostable_item_falls_through_to_the_empty_hand_interaction() {
        let (fixture, player) = composter_fixture("composter_non_compostable");
        give_in_hand(&player, &ItemStack::new(&vanilla_items::STONE));

        let behavior = ComposterBlock::new(&vanilla_blocks::COMPOSTER);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        assert_eq!(
            behavior.use_item_on(
                composter_state(&fixture, TEST_POS),
                &fixture.world,
                TEST_POS,
                &player,
                InteractionHand::MainHand,
                &composter_hit(),
                &mut inventory,
            ),
            InteractionResult::TryEmptyHandInteraction
        );
        assert_eq!(composter_level(&fixture, TEST_POS), 0);
    }

    #[test]
    fn the_weighted_chance_consumes_the_item_and_keeps_event_data_consistent() {
        let (fixture, player, packets) = listening_composter_fixture("composter_weighted");
        give_in_hand(
            &player,
            &ItemStack::with_count(&vanilla_items::OAK_SAPLING, 2),
        );
        assert_eq!(
            click(&fixture, &player, TEST_POS),
            InteractionResult::Success,
            "level 0 always accepts a layer"
        );
        assert_eq!(composter_level(&fixture, TEST_POS), 1);
        assert_eq!(
            click(&fixture, &player, TEST_POS),
            InteractionResult::Success,
            "level 1 still accepts a compostable item"
        );
        let final_level = composter_level(&fixture, TEST_POS);
        assert!(
            (1..=2).contains(&final_level),
            "the weighted roll adds 0 or 1 layers, got {final_level}"
        );
        assert!(
            hand_item(&player).is_empty(),
            "vanilla consumes the item regardless of the roll"
        );
        let data = level_events(&packets)
            .into_iter()
            .rev()
            .find(|(event, _, _)| *event == COMPOSTER_FILL_EVENT)
            .map(|(_, _, data)| data)
            .expect("the second click emits a fill event");
        assert_eq!(
            data,
            i32::from(final_level != 1),
            "the event data tracks whether a layer was added"
        );
    }

    #[test]
    fn a_full_composter_gives_up_its_bone_meal() {
        let (fixture, player, packets) = listening_composter_fixture("composter_extract");
        place_composter(&fixture, TEST_POS, 8);
        give_in_hand(&player, &ItemStack::new(&vanilla_items::STONE));
        packets.lock().clear();

        assert_eq!(
            click(&fixture, &player, TEST_POS),
            InteractionResult::Success
        );

        assert_eq!(composter_level(&fixture, TEST_POS), 0);
        let items = dropped_items(&fixture.world, DVec3::new(4.5, 65.01, 4.5));
        assert_eq!(items.len(), 1, "exactly one bone meal entity is ejected");
        let bone_meal = items[0]
            .downcast_ref::<ItemEntity>()
            .expect("the produce is an item entity")
            .get_item();
        assert!(bone_meal.is(&vanilla_items::BONE_MEAL));
        assert_eq!(bone_meal.count(), 1);
        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].0, sound_events::BLOCK_COMPOSTER_EMPTY.key);
        assert_eq!(sounds[0].2, BLOCK_CENTRE_SOUND_POS);
    }

    #[test]
    fn the_scheduled_tick_ripens_a_full_composter() {
        let (fixture, _player, packets) = listening_composter_fixture("composter_ripen");
        place_composter(&fixture, TEST_POS, 7);
        packets.lock().clear();

        ComposterBlock::new(&vanilla_blocks::COMPOSTER).tick(
            composter_state(&fixture, TEST_POS),
            &fixture.world,
            TEST_POS,
        );

        assert_eq!(composter_level(&fixture, TEST_POS), 8);
        let sounds = played_sounds(&packets);
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].0, sound_events::BLOCK_COMPOSTER_READY.key);
        assert_eq!(sounds[0].2, BLOCK_CENTRE_SOUND_POS);
    }

    #[test]
    fn a_composter_placed_at_level_seven_schedules_its_ripening_tick() {
        let fixture = composter_fixture("composter_on_place").0;
        // `set_block` runs `on_place`, which vanilla uses to re-arm a moved
        // full composter.
        place_composter(&fixture, TEST_POS, 7);

        assert!(
            fixture
                .world
                .has_scheduled_block_tick(TEST_POS, &vanilla_blocks::COMPOSTER),
            "on_place re-arms the ready tick"
        );
    }

    #[test]
    fn a_hopper_above_feeds_one_item_per_transfer() {
        let fixture = hopper_fixture("composter_hopper_fill");
        set_hopper_slot(
            &fixture,
            ABOVE,
            0,
            ItemStack::with_count(&vanilla_items::OAK_SAPLING, 2),
        );
        tick_block_entities(&fixture, 1);

        assert_eq!(composter_level(&fixture, TEST_POS), 1);
        assert_eq!(
            hopper_slot(&fixture, ABOVE, 0).count(),
            1,
            "exactly one item is consumed per transfer"
        );
    }

    /// A hopper below a ready composter: the hopper sits at `TEST_POS` and the
    /// composter at `ABOVE`.
    fn pull_fixture(name: &'static str) -> TestWorld {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        let hopper = vanilla_blocks::HOPPER
            .default_state()
            .set_value(&BlockStateProperties::FACING_HOPPER, Direction::Down);
        assert!(
            fixture
                .world
                .set_block(TEST_POS, hopper, UpdateFlags::UPDATE_ALL)
        );
        place_composter(&fixture, ABOVE, 8);
        fixture
    }

    #[test]
    fn a_hopper_below_takes_the_bone_meal_and_empties_the_composter() {
        let fixture = pull_fixture("composter_hopper_take");
        tick_block_entities(&fixture, 1);

        assert_eq!(composter_level(&fixture, ABOVE), 0);
        let taken = hopper_slot(&fixture, TEST_POS, 0);
        assert!(taken.is(&vanilla_items::BONE_MEAL));
        assert_eq!(taken.count(), 1);
    }

    #[test]
    fn a_failed_pull_still_empties_the_composter() {
        // Five partial stacks leave the hopper not full, so it tries to pull,
        // but nothing merges with bone meal. `SimpleContainer.removeItem` has
        // already emptied the composter by then.
        let fixture = pull_fixture("composter_hopper_failed_take");
        for slot in 0..5 {
            set_hopper_slot(
                &fixture,
                TEST_POS,
                slot,
                ItemStack::new(&vanilla_items::DIRT),
            );
        }
        tick_block_entities(&fixture, 1);

        assert_eq!(composter_level(&fixture, ABOVE), 0);
        assert!(
            (0..5).all(|slot| hopper_slot(&fixture, TEST_POS, slot).is(&vanilla_items::DIRT)),
            "the bone meal is lost, not merged"
        );
    }

    #[test]
    fn a_hopper_below_an_emptied_composter_pulls_nothing() {
        let fixture = pull_fixture("composter_hopper_empty");
        place_composter(&fixture, ABOVE, 0);
        tick_block_entities(&fixture, 3);

        let taken = hopper_slot(&fixture, TEST_POS, 0);
        assert!(
            taken.is_empty(),
            "an empty composter exposes no items to the hopper below"
        );
    }

    /// Vanilla `DropperBlock.dispenseFrom` for a full facing: the composter's
    /// `InputContainer` accepts one compostable item through its top face.
    #[test]
    fn a_dropper_above_the_composter_feeds_it() {
        let fixture = hopper_fixture("composter_dropper_fill");
        let dropper_state = vanilla_blocks::DROPPER
            .default_state()
            .set_value(&BlockStateProperties::FACING, Direction::Down)
            .set_value(&BlockStateProperties::TRIGGERED, true);
        assert!(
            fixture
                .world
                .set_block(ABOVE, dropper_state, UpdateFlags::UPDATE_ALL)
        );
        set_hopper_slot(
            &fixture,
            ABOVE,
            0,
            ItemStack::new(&vanilla_items::OAK_SAPLING),
        );

        DropperBlock::new(&vanilla_blocks::DROPPER).tick(
            composter_state(&fixture, ABOVE),
            &fixture.world,
            ABOVE,
        );

        assert_eq!(composter_level(&fixture, TEST_POS), 1);
        assert!(
            hopper_slot(&fixture, ABOVE, 0).is_empty(),
            "the dropper's slot is emptied into the composter"
        );
    }

    /// Vanilla keeps the item inside a dropper facing a level-7 composter: the
    /// `EmptyContainer` rejects insertion but is still a container, so no
    /// ejection happens.
    #[test]
    fn a_dropper_facing_a_level_seven_composter_keeps_its_item() {
        let fixture = hopper_fixture("composter_dropper_full");
        place_composter(&fixture, TEST_POS, 7);
        let dropper_state = vanilla_blocks::DROPPER
            .default_state()
            .set_value(&BlockStateProperties::FACING, Direction::Down)
            .set_value(&BlockStateProperties::TRIGGERED, true);
        assert!(
            fixture
                .world
                .set_block(ABOVE, dropper_state, UpdateFlags::UPDATE_ALL)
        );
        set_hopper_slot(
            &fixture,
            ABOVE,
            0,
            ItemStack::new(&vanilla_items::OAK_SAPLING),
        );

        DropperBlock::new(&vanilla_blocks::DROPPER).tick(
            composter_state(&fixture, ABOVE),
            &fixture.world,
            ABOVE,
        );

        assert_eq!(composter_level(&fixture, TEST_POS), 7);
        assert!(
            hopper_slot(&fixture, ABOVE, 0).is(&vanilla_items::OAK_SAPLING),
            "the item stays in the dropper instead of being ejected"
        );
        assert!(
            dropped_items(&fixture.world, DVec3::new(4.5, 64.5, 3.5),).is_empty(),
            "nothing is ejected in front of the composter"
        );
    }

    #[test]
    fn the_comparator_reads_the_fill_level() {
        let fixture = composter_fixture("composter_comparator").0;
        place_composter(&fixture, TEST_POS, 5);
        let behavior =
            BLOCK_BEHAVIORS.get_behavior(vanilla_blocks::COMPOSTER.default_state().get_block());

        assert!(behavior.has_analog_output_signal(composter_state(&fixture, TEST_POS)));
        assert_eq!(
            behavior.get_analog_output_signal(
                composter_state(&fixture, TEST_POS),
                &fixture.world,
                TEST_POS,
                Direction::North,
            ),
            5
        );
    }

    /// Vanilla `useItemOn` still returns `SUCCESS` for a compostable item at
    /// level 7, but the `fillLevel < 7` gate consumes and adds nothing.
    #[test]
    fn a_compostable_click_at_level_seven_changes_nothing() {
        let (fixture, player) = composter_fixture("composter_insert_rejects");
        place_composter(&fixture, TEST_POS, 7);
        give_in_hand(&player, &ItemStack::new(&vanilla_items::OAK_SAPLING));

        let behavior = ComposterBlock::new(&vanilla_blocks::COMPOSTER);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);
        assert_eq!(
            behavior.use_item_on(
                composter_state(&fixture, TEST_POS),
                &fixture.world,
                TEST_POS,
                &player,
                InteractionHand::MainHand,
                &composter_hit(),
                &mut inventory,
            ),
            InteractionResult::Success
        );
        assert_eq!(composter_level(&fixture, TEST_POS), 7);
        assert!(
            hand_item(&player).is(&vanilla_items::OAK_SAPLING),
            "the gate skips stat, feedback and consumption"
        );
    }
}
