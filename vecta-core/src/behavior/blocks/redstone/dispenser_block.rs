//! Dispenser block behavior.
//!
//! A dispenser is a nine-slot container that answers a redstone pulse by handing
//! one item from a random non-empty slot to the behavior registered for that
//! item, defaulting to throwing it out of the block. Vanilla puts this pulse
//! logic on `DispenserBlock` and has `DropperBlock` extend it, so the dropper
//! here reuses these functions and overrides only the parts it changes.

use std::slice;
use std::sync::{Arc, OnceLock, Weak};

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::block_entity_type::BlockEntityTypeRef;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::{
    BlockStateProperties, BoolProperty, Direction, EnumProperty,
};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events;
use vecta_registry::stat::custom::CustomStatRef;
use vecta_registry::vanilla_game_events;
use vecta_registry::{vanilla_block_entity_types, vanilla_custom_stats};
use vecta_utils::random::Random as _;
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, BlockStateId, Downcast as _, translations};

use super::dispenser::dispense::{
    BlockSource, DEFAULT_BEHAVIOR, DispenseBehaviorLock, DispenseBehaviorRegistry,
    DispenseItemBehavior,
};
use super::dispenser::equipment::EQUIPMENT_BEHAVIOR;
use crate::behavior::InventoryAccess;
use crate::behavior::block::{BlockBehavior, BlockEntityCreation};
use crate::behavior::context::{BlockHitResult, BlockPlaceContext, InteractionResult};
use crate::block_entity::container_identity::ContainerIdentity;
use crate::block_entity::entities::{DispenserBlockEntity, DropperBlockEntity};
use crate::block_entity::{BLOCK_ENTITIES, BlockEntityTicker, SharedBlockEntity};
use crate::entity::Entity as _;
use crate::inventory::container::calculate_redstone_signal_from_container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef};
use crate::inventory::menu::kinds::dispenser_menu;
use crate::player::Player;
use crate::world::game_event::GameEventContext;
use crate::world::{LevelReader, SignalGetter as _, World};

pub(super) const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING;
pub(super) const TRIGGERED: &BoolProperty = &BlockStateProperties::TRIGGERED;

/// Vanilla `DispenserBlock.TRIGGER_DURATION`: the scheduled delay between the
/// redstone edge and the dispense.
pub(super) const TRIGGER_DURATION: i32 = 4;

/// Vanilla `DispenserBlock.DISPENSER_REGISTRY`, built once by
/// [`init_behaviors`](crate::behavior::init_behaviors).
pub(crate) static DISPENSE_BEHAVIORS: DispenseBehaviorLock = DispenseBehaviorLock(OnceLock::new());

/// The container-backed block entity a dispenser-shaped block opens.
///
/// Vanilla reaches this through `DispenserBlockEntity` and its dropper subclass;
/// this trait is the same seam, so the shared menu path works for both.
pub(super) trait DispenserLikeBlockEntity {
    /// The custom name and lock of this container.
    fn container_identity(&self) -> &ContainerIdentity;

    /// Vanilla `RandomizableContainerBlockEntity.canOpen`.
    fn can_open_container(&self, player: &Player) -> bool;

    /// Rolls any pending loot table before the menu is shown.
    fn unpack_container_loot(&self, player: Option<&Player>);
}

impl DispenserLikeBlockEntity for DispenserBlockEntity {
    fn container_identity(&self) -> &ContainerIdentity {
        self.identity()
    }

    fn can_open_container(&self, player: &Player) -> bool {
        self.can_open(player)
    }

    fn unpack_container_loot(&self, player: Option<&Player>) {
        self.unpack_loot_table(player);
    }
}

impl DispenserLikeBlockEntity for DropperBlockEntity {
    fn container_identity(&self) -> &ContainerIdentity {
        self.identity()
    }

    fn can_open_container(&self, player: &Player) -> bool {
        self.can_open(player)
    }

    fn unpack_container_loot(&self, player: Option<&Player>) {
        self.unpack_loot_table(player);
    }
}

/// Vanilla dispenser block behavior.
#[block_behavior]
pub struct DispenserBlock {
    block: BlockRef,
}

impl DispenserBlock {
    /// Creates a new dispenser block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    /// Vanilla `DispenserBlock.dispenseFrom`: one item from a random non-empty
    /// slot, handed to the behavior registered for it.
    fn dispense_from(world: &Arc<World>, state: BlockStateId, pos: BlockPos) {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return;
        };
        let Some(dispenser) = block_entity.downcast_ref::<DispenserBlockEntity>() else {
            log::warn!(
                "Ignoring dispensing attempt for Dispenser without matching block entity at {pos:?}"
            );
            return;
        };
        let Some(slot) = dispenser
            .get_random_slot(|bound| world.random().with_level(|rng| rng.next_i32_bounded(bound)))
        else {
            // Vanilla plays the empty-dispenser sound and fires `BLOCK_ACTIVATE`.
            world.level_event(level_events::SOUND_DISPENSER_FAIL, pos, 0, None);
            world.game_event(
                &vanilla_game_events::BLOCK_ACTIVATE,
                pos,
                &GameEventContext::new(None, Some(state)),
            );
            return;
        };

        let Some(container) = ContainerRef::from_block_entity(Arc::clone(&block_entity)) else {
            return;
        };
        let id = container.container_id();
        let stack = {
            let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
            guard
                .get(id)
                .map_or_else(ItemStack::empty, |locked| locked.get_item(slot).clone())
        };

        let source = BlockSource { world, pos, state };
        let behavior = Self::get_dispense_method(&stack);
        let remaining = behavior.dispense(&source, stack);
        let mut guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        guard.set_item(id, slot, remaining);
    }

    /// Vanilla `DispenserBlock.getDispenseMethod`: the registry entry, else
    /// `getDefaultDispenseMethod`.
    ///
    /// Vanilla first rejects items disabled by the level's enabled feature
    /// flags; Vecta has no feature-flag support yet, so that branch is absent
    /// rather than faked.
    fn get_dispense_method(stack: &ItemStack) -> &'static dyn DispenseItemBehavior {
        DISPENSE_BEHAVIORS.get(stack.item()).unwrap_or_else(|| {
            if stack.get_equippable().is_some() {
                &EQUIPMENT_BEHAVIOR
            } else {
                &DEFAULT_BEHAVIOR
            }
        })
    }

    /// Vanilla `DispenserBlock.getStateForPlacement`: the block faces away from
    /// the player, so it points where the player is looking.
    pub(super) fn placement_state(
        block: BlockRef,
        context: &BlockPlaceContext<'_>,
    ) -> BlockStateId {
        block
            .default_state()
            .set_value(FACING, context.get_nearest_looking_direction().opposite())
            .set_value(TRIGGERED, false)
    }

    /// Vanilla `DispenserBlock.neighborChanged`: the block above counts too, a
    /// rising edge schedules the pulse, a falling edge clears the flag.
    pub(super) fn neighbor_changed(
        block: BlockRef,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
    ) {
        let should_trigger =
            world.has_neighbor_signal(pos) || world.has_neighbor_signal(pos.above());
        let is_triggered = state.get_value(TRIGGERED);
        if should_trigger && !is_triggered {
            world.schedule_block_tick_default(pos, block, TRIGGER_DURATION);
            world.set_block(
                pos,
                state.set_value(TRIGGERED, true),
                UpdateFlags::UPDATE_CLIENTS,
            );
        } else if !should_trigger && is_triggered {
            world.set_block(
                pos,
                state.set_value(TRIGGERED, false),
                UpdateFlags::UPDATE_CLIENTS,
            );
        }
    }

    /// Vanilla `DispenserBlock.useWithoutItem`: opens the nine-slot menu and
    /// awards the matching inspect statistic.
    pub(super) fn open_menu(
        world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        entity: &SharedBlockEntity,
        block_entity: &dyn DispenserLikeBlockEntity,
        default_name: impl FnOnce() -> TextComponent,
        stat: CustomStatRef,
    ) -> InteractionResult {
        let display_name = block_entity.container_identity().display_name(default_name);
        if !block_entity.can_open_container(player) {
            player.close_container();
            if !player.is_spectator() {
                ContainerIdentity::send_locked_notification(world, player, pos, display_name);
            }
            player.award_custom_stat(stat);
            return InteractionResult::Success;
        }

        block_entity.unpack_container_loot(Some(player));

        let Some(container) = ContainerRef::from_block_entity(Arc::clone(entity)) else {
            return InteractionResult::Success;
        };
        let inventory = Arc::clone(&player.inventory);
        player.open_menu(display_name, move |context| {
            dispenser_menu(inventory, context.container_id, container)
        });

        player.award_custom_stat(stat);
        InteractionResult::Success
    }

    /// Vanilla `AbstractContainerMenu.getRedstoneSignalFromBlockEntity`.
    pub(super) fn analog_signal(
        world: &dyn LevelReader,
        pos: BlockPos,
        unpack: impl FnOnce(),
    ) -> i32 {
        let Some(block_entity) = world.get_block_entity(pos) else {
            return 0;
        };
        unpack();
        let Some(container) = ContainerRef::from_block_entity(block_entity) else {
            return 0;
        };
        let guard = ContainerLockGuard::lock_all(slice::from_ref(&container));
        guard
            .get(container.container_id())
            .map_or(0, calculate_redstone_signal_from_container)
    }
}

impl BlockBehavior for DispenserBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(Self::placement_state(self.block, context))
    }

    fn handle_neighbor_changed(
        &self,
        state: BlockStateId,
        world: &Arc<World>,
        pos: BlockPos,
        _source_block: BlockRef,
        _moved_by_piston: bool,
    ) {
        Self::neighbor_changed(self.block, state, world, pos);
    }

    fn tick(&self, state: BlockStateId, world: &Arc<World>, pos: BlockPos) {
        // Vanilla `DispenserBlock.tick` dispenses on the scheduled tick.
        Self::dispense_from(world, state, pos);
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
            return InteractionResult::Success;
        };
        let Some(dispenser) = block_entity.downcast_ref::<DispenserBlockEntity>() else {
            return InteractionResult::Success;
        };

        Self::open_menu(
            world,
            pos,
            player,
            &block_entity,
            dispenser,
            || TextComponent::translated(translations::CONTAINER_DISPENSER.msg()),
            &vanilla_custom_stats::INSPECT_DISPENSER,
        )
    }

    fn new_block_entity(
        &self,
        level: Weak<World>,
        pos: BlockPos,
        state: BlockStateId,
    ) -> BlockEntityCreation {
        BlockEntityCreation::from_registered_factory(BLOCK_ENTITIES.create(
            &vanilla_block_entity_types::DISPENSER,
            level,
            pos,
            state,
        ))
    }

    fn get_block_entity_ticker(
        &self,
        _world: &Arc<World>,
        _state: BlockStateId,
        _block_entity_type: BlockEntityTypeRef,
    ) -> Option<BlockEntityTicker> {
        // Vanilla `DispenserBlock` has no block-entity ticker: the pulse runs on
        // the scheduled block tick.
        None
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
        Self::analog_signal(world, pos, || {
            if let Some(entity) = world.get_block_entity(pos)
                && let Some(dispenser) = entity.downcast_ref::<DispenserBlockEntity>()
            {
                dispenser.unpack_loot_table(None);
            }
        })
    }
}

/// Builds the dispense behavior registry once.
///
/// Called from [`init_behaviors`](crate::behavior::init_behaviors) after the
/// block behavior registry, so a dispense behavior may look blocks up.
pub(crate) fn init_dispense_behaviors() {
    DISPENSE_BEHAVIORS.0.get_or_init(|| {
        let mut registry = DispenseBehaviorRegistry::new();
        register_dispense_behaviors(&mut registry);
        registry
    });
}

/// Builds the dispense behavior registry.
///
/// Vanilla fills `DISPENSER_REGISTRY` from the item classes; Vecta registers
/// items here instead, so adding a behavior never touches the dispense tick.
pub(crate) fn register_dispense_behaviors(registry: &mut DispenseBehaviorRegistry) {
    register_projectile_dispense_behaviors(registry);
    register_bucket_dispense_behaviors(registry);
    register_ignition_dispense_behaviors(registry);
    register_growth_dispense_behaviors(registry);
    register_bottle_dispense_behaviors(registry);

    // W04-035 (bonemeal, ignition, bottles, shears) adds its items here. Until
    // they exist every other item falls through to the component-based default
    // selection.
}

/// Vanilla `DispenseItemBehavior.bootStrap` bucket registrations.
///
/// The three filled buckets share one behavior: it asks the item for its
/// `DispensibleContainerItem` capability, so water, lava and powder snow keep
/// their own placement rules instead of being special-cased here.
fn register_bucket_dispense_behaviors(registry: &mut DispenseBehaviorRegistry) {
    use vecta_registry::vanilla_items;

    use super::dispenser::bucket::{EmptyBucketDispenseBehavior, FilledBucketDispenseBehavior};

    for filled in [
        &*vanilla_items::WATER_BUCKET,
        &*vanilla_items::LAVA_BUCKET,
        &*vanilla_items::POWDER_SNOW_BUCKET,
    ] {
        registry.register(filled, FilledBucketDispenseBehavior);
    }
    registry.register(&vanilla_items::BUCKET, EmptyBucketDispenseBehavior);
}

/// Vanilla `DispenseItemBehavior.bootStrap` bottle registrations.
fn register_bottle_dispense_behaviors(registry: &mut DispenseBehaviorRegistry) {
    use vecta_registry::vanilla_items;

    use super::dispenser::bottle::GlassBottleDispenseBehavior;

    registry.register(&vanilla_items::GLASS_BOTTLE, GlassBottleDispenseBehavior);
}

/// Vanilla `DispenseItemBehavior.bootStrap` growth registrations.
fn register_growth_dispense_behaviors(registry: &mut DispenseBehaviorRegistry) {
    use vecta_registry::vanilla_items;

    use super::dispenser::bonemeal::BoneMealDispenseBehavior;

    registry.register(&vanilla_items::BONE_MEAL, BoneMealDispenseBehavior);
}

/// Vanilla `DispenseItemBehavior.bootStrap` ignition registrations.
///
/// `Items.FIRE_CHARGE` is a projectile registration in vanilla and stays
/// unregistered until a `SmallFireball` entity exists, so a dispensed fire
/// charge is still thrown out instead of lighting the block directly.
fn register_ignition_dispense_behaviors(registry: &mut DispenseBehaviorRegistry) {
    use vecta_registry::vanilla_items;

    use super::dispenser::flint_and_steel::FlintAndSteelDispenseBehavior;

    registry.register(
        &vanilla_items::FLINT_AND_STEEL,
        FlintAndSteelDispenseBehavior,
    );
}

/// Vanilla `DispenseItemBehavior.bootStrap` projectile registrations that have a
/// finished projectile entity behind them.
///
/// Missing projectile consumers stay unregistered until their entity behavior
/// is implemented; registering a stub would suppress the default item drop.
fn register_projectile_dispense_behaviors(registry: &mut DispenseBehaviorRegistry) {
    use vecta_registry::vanilla_entities;
    use vecta_registry::vanilla_items;

    use super::dispenser::projectile::{
        DEFAULT_POSITION_OFFSET, DEFAULT_POWER, DEFAULT_UNCERTAINTY, DispenseConfig,
        ProjectileDispenseBehavior, ThrowableItemProjectileFactory,
    };

    /// `ExperienceBottleItem` and `ThrowablePotionItem` both override
    /// `createDispenseConfig` with the same values: half the default uncertainty
    /// and 1.25x the default power, keeping the default position function.
    const THROWN_CONSUMABLE_CONFIG: DispenseConfig = DispenseConfig {
        position_offset: DEFAULT_POSITION_OFFSET,
        uncertainty: DEFAULT_UNCERTAINTY * 0.5,
        power: DEFAULT_POWER * 1.25,
        override_dispense_event: None,
    };
    use crate::entity::entities::{
        SnowballEntity, SplashPotionEntity, ThrownEggEntity, ThrownExperienceBottleEntity,
    };
    use crate::entity::next_entity_id;

    registry.register(
        &vanilla_items::SNOWBALL,
        ProjectileDispenseBehavior::new(
            DispenseConfig::DEFAULT,
            ThrowableItemProjectileFactory::new(|world, position| {
                SnowballEntity::new(
                    &vanilla_entities::SNOWBALL,
                    next_entity_id(),
                    position,
                    Arc::downgrade(world),
                )
            }),
        ),
    );

    // Vanilla registers every `EggItem`; the three colours share one entity type
    // and carry their chicken variant as an item component. `EggItem` does not
    // override `createDispenseConfig`, so the dispenser uses the default power.
    for egg in [
        &*vanilla_items::EGG,
        &*vanilla_items::BLUE_EGG,
        &*vanilla_items::BROWN_EGG,
    ] {
        registry.register(
            egg,
            ProjectileDispenseBehavior::new(
                DispenseConfig::DEFAULT,
                ThrowableItemProjectileFactory::new(|world, position| {
                    ThrownEggEntity::new(
                        &vanilla_entities::EGG,
                        next_entity_id(),
                        position,
                        Arc::downgrade(world),
                    )
                }),
            ),
        );
    }

    registry.register(
        &vanilla_items::EXPERIENCE_BOTTLE,
        ProjectileDispenseBehavior::new(
            THROWN_CONSUMABLE_CONFIG,
            ThrowableItemProjectileFactory::new(|world, position| {
                ThrownExperienceBottleEntity::new(
                    &vanilla_entities::EXPERIENCE_BOTTLE,
                    next_entity_id(),
                    position,
                    Arc::downgrade(world),
                )
            }),
        ),
    );

    registry.register(
        &vanilla_items::SPLASH_POTION,
        ProjectileDispenseBehavior::new(
            THROWN_CONSUMABLE_CONFIG,
            ThrowableItemProjectileFactory::new(|world, position| {
                SplashPotionEntity::new(
                    &vanilla_entities::SPLASH_POTION,
                    next_entity_id(),
                    position,
                    Arc::downgrade(world),
                )
            }),
        ),
    );
}

#[cfg(test)]
mod tests {
    mod bonemeal;
    mod bottle;
    mod bucket;
    mod egg;
    mod empty_bucket;
    mod equipment;
    mod experience_bottle;
    mod flint_and_steel;
    mod persistence;
    mod potion;
    mod projectile;
    use std::ptr;
    use std::slice;
    use std::sync::Arc;

    use glam::DVec3;
    use vecta_registry::RegistryEntry as _;
    use vecta_registry::entity_type::EntityTypeRef;
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::level_events::{
        PARTICLES_SHOOT_SMOKE, SOUND_DISPENSER_DISPENSE, SOUND_DISPENSER_FAIL,
        SOUND_DISPENSER_PROJECTILE_LAUNCH,
    };
    use vecta_registry::stat::vanilla_stat_types;
    use vecta_registry::{
        vanilla_blocks, vanilla_custom_stats, vanilla_entities, vanilla_items, vanilla_menu_types,
    };
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos, WorldAabb};

    use super::super::dispenser::dispense::DefaultDispenseItemBehavior;
    use super::*;
    use crate::behavior::context::{PlacementOrientation, PlacementSource};
    use crate::block_entity::SharedBlockEntity;
    use crate::block_entity::entities::{CHEST_SLOTS, DISPENSER_SLOTS};
    use crate::bootstrap::init_globals;
    use crate::entity::entities::{ItemEntity, SnowballEntity};
    use crate::entity::{Projectile as _, ThrowableItemProjectile as _};
    use crate::entity::{SharedEntity, next_entity_id};
    use crate::player::{Player, ResetReason};
    use crate::test_support::{
        SentPackets, TestPlayerBuilder, TestWorld, dropped_items, fresh_test_world,
        insert_ready_full_chunk, level_events as sent_level_events, opened_screens,
        recording_connection,
    };

    const TEST_POS: BlockPos = BlockPos::new(4, 64, 4);

    /// Vanilla `DispenserBlock.TRIGGER_DURATION`, restated here so the test does
    /// not read the implementation's own constant.
    const VANILLA_TRIGGER_DURATION: i64 = 4;

    /// Vanilla `DefaultDispenseItemBehavior.DEFAULT_ACCURACY`.
    const VANILLA_DEFAULT_ACCURACY: f64 = 6.0;

    fn dispenser_fixture(name: &'static str, facing: Direction) -> (TestWorld, BlockPos) {
        init_globals();
        let fixture = fresh_test_world(name);
        insert_ready_full_chunk(&fixture.world, ChunkPos::from_block_pos(TEST_POS));
        let state = vanilla_blocks::DISPENSER
            .default_state()
            .set_value(FACING, facing)
            .set_value(TRIGGERED, false);
        assert!(
            fixture
                .world
                .set_block(TEST_POS, state, UpdateFlags::UPDATE_ALL)
        );
        assert!(
            fixture.world.get_block_entity(TEST_POS).is_some(),
            "a placed dispenser has a block entity"
        );
        (fixture, TEST_POS)
    }

    /// A dispenser fixture whose viewer receives packets, so the level events and
    /// the open-screen can be inspected instead of assumed.
    fn listening_dispenser_fixture(
        name: &'static str,
        facing: Direction,
    ) -> (TestWorld, BlockPos, Arc<Player>, SentPackets) {
        let (fixture, pos) = dispenser_fixture(name, facing);
        let (connection, packets) = recording_connection();
        let player =
            TestPlayerBuilder::new(Arc::clone(&fixture.world), "DispenserFan", next_entity_id())
                .connection(connection)
                .build();
        player
            .try_set_position(DVec3::new(4.5, 65.0, 4.5))
            .expect("the dispenser's chunk is loaded, so the player can stand on it");
        assert!(
            fixture
                .world
                .add_player(Arc::clone(&player), ResetReason::InitialJoin),
            "the listener has to be tracked for the level events to reach it"
        );
        packets.lock().clear();
        (fixture, pos, player, packets)
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

    fn total_items(fixture: &TestWorld, pos: BlockPos, slots: usize) -> i32 {
        (0..slots)
            .map(|slot| item_at(fixture, pos, slot).count())
            .sum()
    }

    fn place_chest(fixture: &TestWorld, pos: BlockPos) {
        assert!(fixture.world.set_block(
            pos,
            vanilla_blocks::CHEST.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
    }

    fn dispenser_at(fixture: &TestWorld, pos: BlockPos) -> SharedBlockEntity {
        fixture
            .world
            .get_block_entity(pos)
            .unwrap_or_else(|| panic!("{pos:?} should hold a dispenser block entity"))
    }

    fn triggered(fixture: &TestWorld, pos: BlockPos) -> bool {
        fixture.world.get_block_state(pos).get_value(TRIGGERED)
    }

    /// Vanilla `DispenserBlock.tick` body, reached through the block behavior.
    fn pulse(behavior: &DispenserBlock, fixture: &TestWorld, pos: BlockPos) {
        behavior.tick(fixture.world.get_block_state(pos), &fixture.world, pos);
    }

    /// Item entities near the centre of `pos`, which is where a dispenser
    /// spawns.
    fn ejected_items(fixture: &TestWorld, pos: BlockPos) -> Vec<SharedEntity> {
        dropped_items(
            &fixture.world,
            DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
        )
    }

    /// Projectiles of `entity_type` near the centre of `pos`, which is where a
    /// dispenser launches them.
    fn projectiles_of_type(
        fixture: &TestWorld,
        pos: BlockPos,
        entity_type: EntityTypeRef,
    ) -> Vec<SharedEntity> {
        fixture.world.get_entities_in_aabb_matching(
            &WorldAabb::of_size(
                DVec3::new(
                    f64::from(pos.x()) + 0.5,
                    f64::from(pos.y()) + 0.5,
                    f64::from(pos.z()) + 0.5,
                ),
                3.0,
                3.0,
                3.0,
            ),
            |entity| entity.entity_type() == entity_type,
        )
    }

    fn dispenser_hit() -> BlockHitResult {
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
    fn a_dispenser_has_no_block_entity_ticker_and_creates_its_own_entity() {
        let (fixture, _pos) = dispenser_fixture("dispenser_ticker", Direction::Down);
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
        let state = fixture.world.get_block_state(TEST_POS);

        assert_eq!(
            fixture.world.block_entity_tickers().registered_len(),
            0,
            "vanilla `DispenserBlock` has no ticker: the pulse runs on the scheduled block tick"
        );
        assert!(
            behavior
                .get_block_entity_ticker(
                    &fixture.world,
                    state,
                    &vanilla_block_entity_types::DISPENSER
                )
                .is_none()
        );
        let block_entity = dispenser_at(&fixture, TEST_POS);
        assert!(
            block_entity
                .downcast_ref::<DispenserBlockEntity>()
                .is_some(),
            "the dispenser block creates a dispenser block entity"
        );
        assert_eq!(
            block_entity.get_type().key,
            vanilla_block_entity_types::DISPENSER.key,
            "a dispenser must not reuse the dropper's block entity type"
        );
    }

    #[test]
    fn a_dispenser_faces_away_from_the_player_look_direction() {
        let (fixture, _pos) = dispenser_fixture("dispenser_placement", Direction::Down);
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
        let mut stack = ItemStack::new(&vanilla_items::DISPENSER);

        let mut placement_facing = |rotation: f32, pitch: f32| {
            let source = PlacementSource::direct(
                None,
                InteractionHand::MainHand,
                &mut stack,
                PlacementOrientation::Player { rotation, pitch },
                false,
            );
            let hit = BlockHitResult {
                location: DVec3::new(4.5, 64.5, 4.5),
                direction: Direction::Up,
                block_pos: TEST_POS,
                miss: false,
                inside: false,
                world_border_hit: false,
            };
            let context = BlockPlaceContext::new(&fixture.world, source, &hit);
            let placed = behavior
                .get_state_for_placement(&context)
                .expect("a dispenser always has a placement state");
            assert!(
                !placed.get_value(TRIGGERED),
                "vanilla places an untriggered dispenser"
            );
            placed.get_value(FACING)
        };

        assert_eq!(
            placement_facing(0.0, 90.0),
            Direction::Up,
            "vanilla `getNearestLookingDirection().getOpposite()` makes a dispenser looked at from above face up"
        );
        assert_eq!(
            placement_facing(180.0, 0.0),
            Direction::South,
            "a player looking north places a dispenser that faces south"
        );
    }

    #[test]
    fn an_empty_dispenser_answers_a_pulse_with_the_failure_sound() {
        let (fixture, pos, _player, packets) =
            listening_dispenser_fixture("dispenser_empty", Direction::Down);
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            sent_level_events(&packets),
            [(SOUND_DISPENSER_FAIL, pos, 0)],
            "vanilla `DispenserBlock.dispenseFrom` sends level event 1001 for an empty dispenser"
        );
        assert_eq!(total_items(&fixture, pos, DISPENSER_SLOTS), 0);
        assert!(ejected_items(&fixture, pos).is_empty());
    }

    #[test]
    fn a_pulse_ejects_one_item_even_with_a_container_in_front() {
        let (fixture, pos, _player, packets) =
            listening_dispenser_fixture("dispenser_eject", Direction::Down);
        let below = pos.below();
        place_chest(&fixture, below);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 2),
        );
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            total_items(&fixture, below, CHEST_SLOTS),
            0,
            "unlike `DropperBlock`, a dispenser never inserts into the container it faces"
        );
        assert_eq!(item_at(&fixture, pos, 0).count(), 1, "one item leaves");
        let ejected = ejected_items(&fixture, pos);
        assert_eq!(ejected.len(), 1, "vanilla spawns exactly one item entity");
        let Some(item_entity) = ejected[0].downcast_ref::<ItemEntity>() else {
            panic!("an ejected item is an item entity");
        };
        assert!(item_entity.get_item().is(&vanilla_items::STONE));
        assert_eq!(item_entity.get_item().count(), 1);

        // `DispenserBlock.getDispensePosition` uses `source.center() + 0.7 *
        // direction`; a vertical facing then loses 2/16 instead of 2.5/16.
        let expected = DVec3::new(4.5, 64.5 - 0.7 - 0.125, 4.5);
        assert!(
            (ejected[0].position() - expected).length() < 1e-9,
            "ejected from {expected:?}, got {:?}",
            ejected[0].position()
        );
        let deviation = 0.017_227_5 * VANILLA_DEFAULT_ACCURACY;
        let velocity = ejected[0].velocity();
        assert!(
            velocity.x.abs() <= deviation && velocity.z.abs() <= deviation,
            "a vertical dispenser only pushes along y, got {velocity:?}"
        );
        assert!(
            (velocity.y - 0.2).abs() <= deviation,
            "vanilla `random.triangle(0.2, 0.0172275 * 6)`, got {velocity:?}"
        );

        assert_eq!(
            sent_level_events(&packets),
            [
                (SOUND_DISPENSER_DISPENSE, pos, 0),
                (
                    PARTICLES_SHOOT_SMOKE,
                    pos,
                    Direction::Down.get_3d_data_value()
                ),
            ],
            "vanilla plays 1000 then the smoke animation 2000 with the facing data"
        );
    }

    #[test]
    fn one_pulse_moves_exactly_one_item_from_nine_full_slots() {
        let (fixture, pos) = dispenser_fixture("dispenser_one_item", Direction::Down);
        for slot in 0..DISPENSER_SLOTS {
            set_item_at(
                &fixture,
                pos,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            total_items(&fixture, pos, DISPENSER_SLOTS),
            9 * 64 - 1,
            "vanilla takes one item from one random slot, not every stack"
        );
        assert_eq!(ejected_items(&fixture, pos).len(), 1);
    }

    #[test]
    fn a_long_powered_signal_dispenses_once_per_rising_edge() {
        let (fixture, pos) = dispenser_fixture("dispenser_signal", Direction::Down);
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 3),
        );
        let now = fixture.world.game_time();

        assert!(fixture.world.set_block(
            pos.east(),
            vanilla_blocks::REDSTONE_BLOCK.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            triggered(&fixture, pos),
            "a rising edge latches `TRIGGERED`"
        );
        assert!(
            fixture
                .world
                .has_scheduled_block_tick(pos, &vanilla_blocks::DISPENSER)
        );

        let early = fixture
            .world
            .begin_scheduled_tick_phase(now + VANILLA_TRIGGER_DURATION - 1, 64);
        assert!(
            early.ticks.is_empty(),
            "vanilla `scheduleTick(pos, this, 4)` does not fire early"
        );
        let due = fixture
            .world
            .begin_scheduled_tick_phase(now + VANILLA_TRIGGER_DURATION, 64);
        assert_eq!(due.ticks.len(), 1);
        assert_eq!(due.ticks[0].pos, pos);
        assert_eq!(due.ticks[0].trigger_tick, now + VANILLA_TRIGGER_DURATION);
        assert!(ptr::eq(
            due.ticks[0].tick_type,
            &raw const vanilla_blocks::DISPENSER
        ));

        pulse(&behavior, &fixture, pos);
        assert_eq!(
            total_items(&fixture, pos, DISPENSER_SLOTS),
            2,
            "one pulse moves one item"
        );

        assert!(fixture.world.set_block(
            pos.east(),
            vanilla_blocks::AIR.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            !triggered(&fixture, pos),
            "the falling edge clears `TRIGGERED`"
        );
        assert!(
            !fixture
                .world
                .has_scheduled_block_tick(pos, &vanilla_blocks::DISPENSER),
            "the falling edge does not schedule a dispense"
        );
    }

    #[test]
    fn a_powered_block_above_the_dispenser_counts_as_a_neighbour_signal() {
        let (fixture, pos) = dispenser_fixture("dispenser_above", Direction::Down);
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
        // A solid block on top, powered from above: the dispenser itself stays
        // unpowered, but vanilla also checks `hasNeighborSignal(pos.above())`.
        assert!(fixture.world.set_block(
            pos.above(),
            vanilla_blocks::STONE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(fixture.world.set_block(
            pos.above().above(),
            vanilla_blocks::REDSTONE_BLOCK.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        assert!(
            !fixture.world.has_neighbor_signal(pos),
            "a redstone block two blocks up must not power the dispenser itself"
        );
        assert!(
            fixture.world.has_neighbor_signal(pos.above()),
            "the block above the dispenser is powered"
        );

        behavior.handle_neighbor_changed(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            &vanilla_blocks::REDSTONE_BLOCK,
            false,
        );

        assert!(
            triggered(&fixture, pos),
            "vanilla `DispenserBlock.neighborChanged` treats a powered block above as a signal"
        );
        assert!(
            fixture
                .world
                .has_scheduled_block_tick(pos, &vanilla_blocks::DISPENSER),
            "the rising edge schedules the pulse"
        );
    }

    #[test]
    fn a_plain_item_falls_through_to_the_default_behavior() {
        let (fixture, _pos) = dispenser_fixture("dispenser_default", Direction::Down);
        let stone = ItemStack::new(&vanilla_items::STONE);
        let resolved =
            ptr::from_ref::<dyn DispenseItemBehavior>(DispenserBlock::get_dispense_method(&stone));
        let expected = ptr::from_ref::<DefaultDispenseItemBehavior>(&DEFAULT_BEHAVIOR);

        assert!(
            ptr::addr_eq(resolved, expected),
            "vanilla `getDispenseMethod` returns `DEFAULT_BEHAVIOR` for an unregistered item, \
             so a stone is thrown out instead of being used"
        );
        // Keep the fixture alive so the world outlives the lookup, which reads
        // the (empty) registry.
        assert!(fixture.world.get_block_entity(TEST_POS).is_some());
    }

    #[test]
    fn a_dispensed_snowball_is_launched_along_the_facing() {
        let (fixture, pos, _player, packets) =
            listening_dispenser_fixture("dispenser_snowball", Direction::South);
        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::SNOWBALL, 2),
        );
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);

        pulse(&behavior, &fixture, pos);

        assert_eq!(
            item_at(&fixture, pos, 0).count(),
            1,
            "a dispensed projectile consumes exactly one item"
        );
        let projectiles = projectiles_of_type(&fixture, pos, &vanilla_entities::SNOWBALL);
        assert_eq!(
            projectiles.len(),
            1,
            "vanilla `ProjectileDispenseBehavior` spawns one projectile"
        );
        let Some(snowball) = projectiles[0].downcast_ref::<SnowballEntity>() else {
            panic!("the dispensed entity is the snowball projectile");
        };
        assert!(
            snowball.get_item().is(&vanilla_items::SNOWBALL),
            "the projectile carries the item it was dispensed from"
        );
        assert_eq!(
            snowball.get_item().count(),
            1,
            "vanilla keeps a single-item copy on the projectile"
        );
        assert!(
            snowball.get_owner().is_none(),
            "a dispenser is not an entity, so vanilla gives the projectile no owner"
        );

        // Vanilla `DispenserBlock.getDispensePosition(source, 0.7, Vec3(0, 0.1, 0))`.
        let expected = DVec3::new(4.5, 64.5 + 0.1, 4.5 + 0.7);
        assert!(
            (projectiles[0].position() - expected).length() < 1e-9,
            "launched from {expected:?}, got {:?}",
            projectiles[0].position()
        );

        // `DispenseConfig::DEFAULT` is power 1.1 with uncertainty 6, and the
        // jitter is `0.0172275 * 6` per axis before the power scale.
        let velocity = projectiles[0].velocity();
        assert!(
            (velocity.z - 1.1).abs() <= 0.12
                && velocity.x.abs() <= 0.12
                && velocity.y.abs() <= 0.12,
            "a south-facing dispenser launches at power 1.1 with a small spread, got {velocity:?}"
        );

        assert_eq!(
            sent_level_events(&packets),
            [
                (SOUND_DISPENSER_PROJECTILE_LAUNCH, pos, 0),
                (
                    PARTICLES_SHOOT_SMOKE,
                    pos,
                    Direction::South.get_3d_data_value()
                ),
            ],
            "vanilla overrides only `playSound`; the inherited smoke animation still plays"
        );
    }

    #[test]
    fn opening_a_dispenser_sends_the_generic_3x3_menu_and_awards_the_inspect_stat() {
        let (fixture, pos, player, packets) =
            listening_dispenser_fixture("dispenser_menu", Direction::Down);
        let mut inventory =
            InventoryAccess::new(Arc::clone(&player.inventory), InteractionHand::MainHand);

        let result = DispenserBlock::new(&vanilla_blocks::DISPENSER).use_without_item(
            fixture.world.get_block_state(pos),
            &fixture.world,
            pos,
            &player,
            &dispenser_hit(),
            &mut inventory,
        );

        assert_eq!(result, InteractionResult::Success);
        assert!(player.has_container_open());
        let screens = opened_screens(&packets);
        assert_eq!(
            screens.len(),
            1,
            "opening a dispenser sends one open-screen"
        );
        assert_eq!(
            screens[0].1,
            vanilla_menu_types::GENERIC_3X3.id() as i32,
            "vanilla `DispenserMenu` uses the generic 3x3 menu type"
        );
        assert_eq!(
            player
                .get_stat(vanilla_stat_types::CUSTOM.get(&vanilla_custom_stats::INSPECT_DISPENSER)),
            1,
            "vanilla awards `inspect_dispenser` for a dispenser"
        );
    }

    #[test]
    fn a_dispenser_reports_its_fullness_to_a_comparator() {
        let (fixture, pos) = dispenser_fixture("dispenser_comparator", Direction::Down);
        let behavior = DispenserBlock::new(&vanilla_blocks::DISPENSER);
        let state = fixture.world.get_block_state(pos);
        let signal = |fixture: &TestWorld| {
            behavior.get_analog_output_signal(state, fixture.world.as_ref(), pos, Direction::Down)
        };

        assert!(behavior.has_analog_output_signal(state));
        assert_eq!(signal(&fixture), 0, "an empty dispenser emits nothing");

        set_item_at(
            &fixture,
            pos,
            0,
            ItemStack::with_count(&vanilla_items::STONE, 64),
        );
        assert_eq!(
            signal(&fixture),
            2,
            "one full slot out of nine is `floor(14 / 9) + 1`"
        );

        for slot in 1..DISPENSER_SLOTS {
            set_item_at(
                &fixture,
                pos,
                slot,
                ItemStack::with_count(&vanilla_items::STONE, 64),
            );
        }
        assert_eq!(signal(&fixture), 15, "a full dispenser emits a full signal");
    }
}
