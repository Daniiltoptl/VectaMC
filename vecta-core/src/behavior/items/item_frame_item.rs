//! Vanilla `ItemFrameItem` placement of hanging item frames.

use vecta_macros::item_behavior;
use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::items::ItemRef;
use vecta_registry::{vanilla_entities, vanilla_game_events, vanilla_items};
use vecta_utils::{BlockPos, Direction};

use crate::behavior::{InteractionResult, ItemBehavior, UseOnContext};
use crate::entity::block_attached_entity::BlockAttachedEntity;
use crate::entity::entities::objects::display_ui::create_item_frame;
use crate::entity::{add_spawned_entity, apply_implicit_item_stack_components};
use crate::player::Player;
use crate::world::game_event::GameEventContext;

/// Behavior for the `item_frame` and `glow_item_frame` registry entries.
#[item_behavior(class = "ItemFrameItem")]
pub struct ItemFrameItem;

impl ItemFrameItem {
    /// Maps a hanging frame item to the entity type it places.
    ///
    /// Vanilla hardcodes this in the item constructors
    /// (`new ItemFrameItem(EntityTypes.ITEM_FRAME, ...)` and
    /// `new ItemFrameItem(EntityTypes.GLOW_ITEM_FRAME, ...)`), so the mapping is
    /// mirrored here rather than invented as registry data.
    fn entity_type(item: ItemRef) -> Option<EntityTypeRef> {
        if item == &*vanilla_items::ITEM_FRAME {
            Some(&vanilla_entities::ITEM_FRAME)
        } else if item == &*vanilla_items::GLOW_ITEM_FRAME {
            Some(&vanilla_entities::GLOW_ITEM_FRAME)
        } else {
            None
        }
    }

    /// Vanilla `ItemFrameItem.mayPlace`: build height and adventure predicates.
    fn may_place(player: &Player, face: Direction, stack: &ItemStack, block_pos: BlockPos) -> bool {
        let world = player.get_world();
        !world.is_outside_build_height(block_pos.y())
            && player.may_use_item_at(block_pos, face, stack)
    }
}

impl ItemBehavior for ItemFrameItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        let clicked_pos = context.hit_result.block_pos;
        let clicked_face = context.hit_result.direction;
        let block_pos = clicked_pos.relative(clicked_face);
        let player = context.player.as_ref();
        let stack = context.inv.with_item(|item| item.clone());
        let Some(entity_type) = Self::entity_type(stack.item()) else {
            return InteractionResult::Pass;
        };

        if !Self::may_place(player, clicked_face, &stack, block_pos) {
            return InteractionResult::Fail;
        }

        let Some(entity) = create_item_frame(entity_type, block_pos, clicked_face, context.world)
        else {
            log::warn!(
                "no item frame entity factory for {} while placing",
                entity_type.key
            );
            return InteractionResult::Fail;
        };
        apply_implicit_item_stack_components(&entity, &stack);

        let survives = entity
            .as_block_attached_entity()
            .is_some_and(BlockAttachedEntity::survives);
        if !survives {
            return InteractionResult::Consume;
        }

        if let Some(frame) = entity.as_item_frame() {
            frame.play_placement_sound();
        }
        context.world.game_event_at(
            &vanilla_game_events::ENTITY_PLACE,
            entity.position(),
            &GameEventContext::new(Some(player), None),
        );
        if let Err(error) = add_spawned_entity(context.world, entity) {
            log::warn!("failed to add placed item frame: {error:?}");
            return InteractionResult::Fail;
        }
        context
            .inv
            .with_item(|item| item.consume_one(player.has_infinite_materials()));
        InteractionResult::Success
    }
}
