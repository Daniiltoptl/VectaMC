//! Solid bucket item behavior implementation.

use std::sync::Arc;

use vecta_macros::item_behavior;
use vecta_registry::{
    blocks::BlockRef, item_stack::ItemStack, sound_event::SoundEventRef, vanilla_game_events,
    vanilla_items,
};
use vecta_utils::BlockPos;
use vecta_utils::types::UpdateFlags;

use crate::behavior::items::BlockItem;
use crate::behavior::{
    DispensibleContainerItem, FluidPlacementHit, InteractionResult, ItemBehavior, UseOnContext,
};
use crate::entity::Entity;
use crate::player::Player;
use crate::world::World;
use crate::world::game_event::GameEventContext;
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;

/// Behavior for buckets that place a solid block, such as powder snow.
#[item_behavior]
pub struct SolidBucketItem {
    #[json_arg(vanilla_blocks, json = "block")]
    block: BlockRef,
    #[json_arg(sound_events, json = "place_sound")]
    place_sound: SoundEventRef,
    base: BlockItem,
}

impl SolidBucketItem {
    /// Creates a solid bucket item behavior.
    #[must_use]
    pub const fn new(block: BlockRef, place_sound: SoundEventRef) -> Self {
        Self {
            block,
            place_sound,
            base: BlockItem::new(block),
        }
    }
}

impl ItemBehavior for SolidBucketItem {
    fn use_on(&self, context: &mut UseOnContext) -> InteractionResult {
        let result = self.base.place_with_sound_and_block(
            context.build_place_context(),
            BlockItem::place_block,
            self.place_sound,
        );

        if matches!(result, InteractionResult::Success) && !context.player.has_infinite_materials()
        {
            context
                .inv
                .with_item(|item| *item = ItemStack::new(&vanilla_items::BUCKET));
        }

        result
    }

    fn as_dispensible_container(&self) -> Option<&dyn DispensibleContainerItem> {
        Some(self)
    }
}

impl DispensibleContainerItem for SolidBucketItem {
    fn empty_contents(
        &self,
        user: Option<&Player>,
        world: &Arc<World>,
        pos: BlockPos,
        _hit: Option<FluidPlacementHit>,
    ) -> bool {
        if !world.is_in_valid_bounds(pos) || !world.get_block_state(pos).is_air() {
            return false;
        }

        world.set_block(pos, self.block.default_state(), UpdateFlags::UPDATE_ALL);
        world.game_event(
            &vanilla_game_events::FLUID_PLACE,
            pos,
            &GameEventContext::new(user.map(|player| player as &dyn Entity), None),
        );
        world.play_block_sound(self.place_sound, pos, 1.0, 1.0, user.map(Player::id));
        true
    }
}
