//! Block item behavior implementation.

use vecta_macros::item_behavior;
use vecta_registry::{
    blocks::{BlockRef, block_state_ext::BlockStateExt, shapes::OffsetVoxelShape},
    data_components::vanilla_components::BLOCK_ENTITY_DATA,
    sound_event::SoundEventRef,
    vanilla_blocks, vanilla_game_events,
};
use vecta_utils::{BlockStateId, types::UpdateFlags};

use crate::behavior::{BLOCK_BEHAVIORS, BlockCollisionContext, ItemBehavior};
use crate::entity::Entity;
use crate::fluid::{FluidStateExt as _, get_fluid_state};
use crate::world::game_event::GameEventContext;
use crate::{
    behavior::context::{BlockPlaceContext, InteractionResult, UseOnContext},
    player::Player,
};

pub(super) enum SurvivalCheck {
    Required,
    Skipped,
}

/// Behavior for items that place blocks.
#[item_behavior]
pub struct BlockItem {
    /// The block this item places.
    #[json_arg(vanilla_blocks, json = "block")]
    pub block: BlockRef,
}

impl BlockItem {
    const PLACE_BLOCK_FLAGS: UpdateFlags = UpdateFlags::UPDATE_ALL_IMMEDIATE;

    /// Creates a new block item behavior for the given block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }

    pub(super) fn place_with(
        &self,
        context: BlockPlaceContext<'_>,
        place_block: impl FnOnce(&BlockPlaceContext<'_>, BlockStateId) -> bool,
    ) -> InteractionResult {
        self.place_with_policy(
            context,
            Some,
            SurvivalCheck::Required,
            place_block,
            self.block.config.sound_type.place_sound,
            |place_context| self.get_state_for_placement(place_context),
        )
    }

    pub(super) fn place_with_sound_and_block(
        &self,
        context: BlockPlaceContext<'_>,
        place_block: impl FnOnce(&BlockPlaceContext<'_>, BlockStateId) -> bool,
        place_sound: SoundEventRef,
    ) -> InteractionResult {
        self.place_with_policy(
            context,
            Some,
            SurvivalCheck::Required,
            place_block,
            place_sound,
            |place_context| self.get_state_for_placement(place_context),
        )
    }

    #[expect(
        clippy::manual_midpoint,
        reason = "Matches vanilla BlockItem::place's sound volume formula"
    )]
    pub(super) fn place_with_policy<'a>(
        &self,
        context: BlockPlaceContext<'a>,
        update_context: impl FnOnce(BlockPlaceContext<'a>) -> Option<BlockPlaceContext<'a>>,
        survival_check: SurvivalCheck,
        place_block: impl FnOnce(&BlockPlaceContext<'a>, BlockStateId) -> bool,
        place_sound: SoundEventRef,
        get_state_for_placement: impl FnOnce(&BlockPlaceContext<'a>) -> Option<BlockStateId>,
    ) -> InteractionResult {
        if !context.can_place() {
            return InteractionResult::Fail;
        }
        let Some(mut context) = update_context(context) else {
            return InteractionResult::Fail;
        };
        let place_pos = context.place_pos();

        let Some(new_state) = get_state_for_placement(&context) else {
            return InteractionResult::Fail;
        };

        let behavior = BLOCK_BEHAVIORS.get_behavior(new_state.get_block());
        if matches!(survival_check, SurvivalCheck::Required)
            && !behavior.can_survive(new_state, context.world.as_ref(), place_pos)
        {
            return InteractionResult::Fail;
        }

        let collision_context = context.player().map_or_else(
            BlockCollisionContext::placement_without_entity,
            |player| {
                BlockCollisionContext::with_position(player.position().y, player.is_descending())
            },
        );
        let collision_shape = OffsetVoxelShape::new(
            behavior.get_collision_shape(
                new_state,
                context.world.as_ref(),
                place_pos,
                collision_context,
            ),
            behavior.get_collision_shape_offset(
                new_state,
                context.world.as_ref(),
                place_pos,
                collision_context,
            ),
        );
        if !context.world.is_unobstructed(collision_shape, place_pos) {
            return InteractionResult::Fail;
        }

        if !place_block(&context, new_state) {
            return InteractionResult::Fail;
        }

        let placed_state = context.world.get_block_state(place_pos);
        if placed_state.get_block() == new_state.get_block() {
            if let Some(block_entity) = context.world.get_block_entity(place_pos) {
                // Loading and component callbacks may enter world/container code;
                // retain a placement snapshot instead of the player's inventory lock.
                let item = context.with_item(|item| item.copy_with_count(item.count()));
                if let Some(data) = item.get(BLOCK_ENTITY_DATA)
                    && data.block_entity_type().key == block_entity.get_type().key
                {
                    block_entity.load_item_custom_data(data.data().as_compound());
                }
                block_entity.apply_components_from_item(&item);
                block_entity.set_changed();
            }
            let placed_behavior = BLOCK_BEHAVIORS.get_behavior(placed_state.get_block());
            placed_behavior.set_placed_by(placed_state, context.world, place_pos, context.source());
        }

        // Play place sound (exclude the placing player, they hear it client-side)
        context.world.play_block_sound(
            place_sound,
            place_pos,
            (self.block.config.sound_type.volume + 1.0) / 2.0,
            self.block.config.sound_type.pitch * 0.8,
            context.player().map(Entity::id),
        );
        context.world.game_event(
            &vanilla_game_events::BLOCK_PLACE,
            place_pos,
            &GameEventContext::new(
                context.player().map(|player| player as &dyn Entity),
                Some(placed_state),
            ),
        );

        let has_infinite_materials = context.player().is_some_and(Player::has_infinite_materials);
        context.with_item_mut(|item| item.consume_one(has_infinite_materials));

        InteractionResult::Success
    }

    /// Places this block using an already constructed placement context.
    pub fn place(&self, context: BlockPlaceContext<'_>) -> InteractionResult {
        self.place_with(context, Self::place_block)
    }

    pub(super) fn place_block(context: &BlockPlaceContext<'_>, state: BlockStateId) -> bool {
        context
            .world
            .set_block(context.place_pos(), state, Self::PLACE_BLOCK_FLAGS)
    }

    pub(super) fn get_state_for_placement(
        &self,
        context: &BlockPlaceContext<'_>,
    ) -> Option<BlockStateId> {
        let behavior = BLOCK_BEHAVIORS.get_behavior(self.block);
        behavior.get_state_for_placement(context)
    }
}

impl ItemBehavior for BlockItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        self.place(context.build_place_context())
    }

    fn can_fit_inside_container_items(&self) -> bool {
        BLOCK_BEHAVIORS
            .get_behavior(self.block)
            .fits_inside_container_items()
    }
}

/// Behavior for double-high block items (doors, tall flowers, etc.).
///
/// Vanilla's `DoubleHighBlockItem` extends `BlockItem` and overrides `placeBlock`
/// to place the upper half block above the lower half.
///
/// The `_block` field is read by the build script via `#[json_arg]` to generate constructor
/// calls from `classes.json`. The actual value is forwarded into `base`.
#[item_behavior]
pub struct DoubleHighBlockItem {
    #[json_arg(vanilla_blocks, json = "block")]
    _block: BlockRef,
    base: BlockItem,
}

impl DoubleHighBlockItem {
    const PREPARE_UPPER_FLAGS: UpdateFlags =
        UpdateFlags::UPDATE_ALL_IMMEDIATE.union(UpdateFlags::UPDATE_KNOWN_SHAPE);

    /// Creates a new double-high block item behavior for the given block.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self {
            _block: block,
            base: BlockItem::new(block),
        }
    }

    fn place_block(context: &BlockPlaceContext<'_>, state: BlockStateId) -> bool {
        let above = context.place_pos().above();
        let above_state = if get_fluid_state(context.world, above).is_water() {
            vanilla_blocks::WATER.default_state()
        } else {
            vanilla_blocks::AIR.default_state()
        };
        let _ = context
            .world
            .set_block(above, above_state, Self::PREPARE_UPPER_FLAGS);

        BlockItem::place_block(context, state)
    }
}

impl ItemBehavior for DoubleHighBlockItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        self.base
            .place_with(context.build_place_context(), Self::place_block)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use glam::DVec3;
    use simdnbt::owned::NbtCompound;
    use text_components::TextComponent;
    use vecta_registry::data_components::components::{BlockEntityData, CustomData};
    use vecta_registry::data_components::vanilla_components::{BLOCK_ENTITY_DATA, CUSTOM_NAME};
    use vecta_registry::item_stack::ItemStack;
    use vecta_registry::{vanilla_block_entity_types, vanilla_blocks, vanilla_items};
    use vecta_utils::types::{InteractionHand, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos, Direction, Downcast as _};

    use super::*;
    use crate::behavior::{BlockHitResult, UseOnContext};
    use crate::block_entity::entities::BarrelBlockEntity;
    use crate::bootstrap::init_globals;
    use crate::test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk};

    #[test]
    fn placing_a_barrel_item_loads_only_matching_block_entity_data_before_the_components() {
        for (payload_type, should_load) in [
            (&vanilla_block_entity_types::BARREL, true),
            (&vanilla_block_entity_types::CHEST, false),
        ] {
            init_globals();
            let fixture = fresh_test_world("block_item_place_custom_data");
            let world = &fixture.world;
            let ground = BlockPos::new(0, 63, 0);
            insert_ready_full_chunk(world, ChunkPos::from_block_pos(ground));
            assert!(world.set_block(
                ground,
                vanilla_blocks::STONE.default_state(),
                UpdateFlags::UPDATE_ALL,
            ));

            let player = TestPlayerBuilder::new(Arc::clone(world), "Builder", 1).build();
            player
                .try_set_position(DVec3::new(2.5, 64.0, 0.5))
                .expect("position the builder");
            player.set_rotation((90.0, 0.0));

            // A loot table is a specialised field with no implicit item component,
            // so only `updateCustomBlockEntityTag` can put it on the new entity.
            let mut payload = NbtCompound::new();
            payload.insert("LootTable", "minecraft:chests/simple_dungeon");
            let mut item = ItemStack::new(&vanilla_items::BARREL);
            item.set(
                BLOCK_ENTITY_DATA,
                BlockEntityData::new(
                    payload_type,
                    CustomData::try_from_compound(payload).expect("payload is valid custom data"),
                ),
            );
            item.set(CUSTOM_NAME, TextComponent::plain("Stored"));
            player
                .inventory
                .lock()
                .set_item_in_hand(InteractionHand::MainHand, item);

            let hit = BlockHitResult {
                location: DVec3::new(0.5, 64.0, 0.5),
                direction: Direction::Up,
                block_pos: ground,
                miss: false,
                inside: false,
                world_border_hit: false,
            };
            let mut context = UseOnContext::new(
                &player,
                InteractionHand::MainHand,
                hit,
                world,
                Arc::clone(&player.inventory),
            );
            assert_eq!(
                BlockItem::new(&vanilla_blocks::BARREL).use_on(&mut context),
                InteractionResult::Success
            );

            let placed_pos = ground.above();
            let block_entity = world
                .get_block_entity(placed_pos)
                .expect("the placed barrel owns a block entity");
            let saved = block_entity.save_custom_only();
            assert_eq!(
                saved.get("LootTable").is_some(),
                should_load,
                "vanilla `updateCustomBlockEntityTag` loads the payload into the real block entity"
            );
            let barrel = block_entity
                .downcast_ref::<BarrelBlockEntity>()
                .expect("the placed block entity is a barrel");
            assert_eq!(
                barrel.identity().custom_name(),
                Some(TextComponent::plain("Stored")),
                "`applyComponentsFromItemStack` then applies the implicit name component"
            );
        }
    }
}
