//! Writable and written book item behaviors.

use std::sync::Arc;

use vecta_macros::item_behavior;
use vecta_protocol::packets::game::COpenBook;
use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_registry::stat::vanilla_stat_types;

use crate::{
    behavior::{InteractionResult, ItemBehavior, UseItemContext},
    command::{
        execution::{CommandSource, CommandTextResolver},
        sender::CommandSender,
    },
};

/// Behavior for the `minecraft:writable_book` item.
///
/// The client opens the edit screen. Additional written content still follows
/// the same server GUI path as a written book.
#[item_behavior]
pub struct WritableBookItem;

impl ItemBehavior for WritableBookItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        use_book(context)
    }
}

/// Behavior for the `minecraft:written_book` item.
#[item_behavior]
pub struct WrittenBookItem;

impl ItemBehavior for WrittenBookItem {
    fn use_item(&self, context: &mut UseItemContext) -> InteractionResult {
        use_book(context)
    }
}

fn use_book(context: &mut UseItemContext) -> InteractionResult {
    // NBT pages can read this inventory, so resolve a snapshot without its lock.
    let unresolved = context.inv.with_item(|stack| {
        stack
            .get(WRITTEN_BOOK_CONTENT)
            .filter(|content| !content.resolved())
            .cloned()
    });
    if let Some(content) = unresolved
        && let Some(server) = context.player.server.upgrade()
    {
        let source = CommandSource::new(CommandSender::Player(Arc::clone(context.player)), server);
        let resolver = CommandTextResolver::with_entity_override(&source, context.player.as_ref());
        let (updated_content, broadcast) = match content.resolve(&resolver) {
            Some(updated_content) => (updated_content, true),
            None => (content.mark_resolved(), false),
        };
        context
            .inv
            .with_item(|stack| stack.set(WRITTEN_BOOK_CONTENT, updated_content));
        if broadcast {
            context.player.broadcast_inventory_changes();
        }
    }

    if context
        .inv
        .with_item(|stack| stack.has(WRITTEN_BOOK_CONTENT))
    {
        context.player.send_packet(COpenBook { hand: context.hand });
    }
    let item = context.inv.with_item(|stack| stack.item());
    context
        .player
        .award_stat(&vanilla_stat_types::ITEM_USED, item);
    InteractionResult::Success
}

#[cfg(test)]
#[path = "books/tests.rs"]
mod tests;
