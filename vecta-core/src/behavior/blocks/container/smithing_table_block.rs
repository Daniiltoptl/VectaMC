//! Smithing table block behavior.
//!
//! Opens the smithing menu when right-clicked without an item.

use std::sync::Arc;

use text_components::TextComponent;
use vecta_macros::block_behavior;
use vecta_registry::blocks::BlockRef;
use vecta_registry::vanilla_custom_stats;
use vecta_utils::{BlockPos, BlockStateId, translations};

use crate::{
    behavior::{
        BlockBehavior, BlockPlaceContext, InteractionResult, InventoryAccess,
        context::BlockHitResult,
    },
    inventory::menu::kinds::smithing,
    player::Player,
    world::World,
};

/// Behavior for the smithing table block.
///
/// `SmithingTableBlock` only overrides the menu provider and the container
/// title; placement, the open flow and the interaction stat come from
/// `CraftingTableBlock`, which Vecta's crafting table behavior already mirrors.
#[block_behavior]
pub struct SmithingTableBlock {
    block: BlockRef,
}

impl SmithingTableBlock {
    /// Creates a new smithing table block behavior.
    #[must_use]
    pub const fn new(block: BlockRef) -> Self {
        Self { block }
    }
}

impl BlockBehavior for SmithingTableBlock {
    fn get_state_for_placement(&self, _context: &BlockPlaceContext<'_>) -> Option<BlockStateId> {
        Some(self.block.default_state())
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
            TextComponent::translated(translations::CONTAINER_UPGRADE.msg()),
            move |context| smithing(inventory, context.container_id, pos, context.world),
        );
        player.award_custom_stat(&vanilla_custom_stats::INTERACT_WITH_SMITHING_TABLE);
        InteractionResult::Success
    }
}
