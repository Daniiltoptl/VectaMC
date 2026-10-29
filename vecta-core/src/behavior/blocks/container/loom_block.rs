//! Loom block behavior.

use std::sync::Arc;

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::blocks::block_state_ext::BlockStateExt;
use vecta_registry::blocks::properties::{BlockStateProperties, Direction, EnumProperty};
use vecta_registry::vanilla_custom_stats;
use vecta_utils::{BlockPos, BlockStateId, translations};

use crate::{
    behavior::{
        BlockBehavior, BlockPlaceContext, InteractionResult, InventoryAccess,
        context::BlockHitResult,
    },
    inventory::menu::kinds::loom,
    player::Player,
    world::World,
};

/// Behavior for the loom block.
#[block_behavior]
pub struct LoomBlock {
    block: BlockRef,
}

const FACING: &EnumProperty<Direction> = &BlockStateProperties::FACING;

impl LoomBlock {
    /// Creates a new loom block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehavior for LoomBlock {
    fn get_state_for_placement(&self, context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        // The loom faces the player who placed it.
        let facing = context.horizontal_direction().opposite();

        Some(self.block.default_state().set_value(FACING, facing))
    }

    fn use_without_item(
        &self,
        _state: BlockStateId,
        _world: &Arc<World>,
        pos: BlockPos,
        player: &Player,
        _hit_result: &BlockHitResult,
        _inv: &mut InventoryAccess,
    ) -> InteractionResult {
        let inventory = Arc::clone(&player.inventory);
        player.open_menu(
            TextComponent::translated(translations::CONTAINER_LOOM.msg()),
            move |context| loom(inventory, context.container_id, pos, context.world),
        );
        player.award_custom_stat(&vanilla_custom_stats::INTERACT_WITH_LOOM);
        InteractionResult::Success
    }
}
