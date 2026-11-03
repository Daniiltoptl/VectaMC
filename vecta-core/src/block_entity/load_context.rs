//! Read access for NBT loads applied to an already attached block entity.

use std::sync::Arc;

use vecta_registry::data_components::vanilla_components::WRITTEN_BOOK_CONTENT;
use vecta_registry::item_stack::ItemStack;
use vecta_utils::BlockPos;

use crate::command::{
    brigadier::CommandSyntaxError,
    execution::{CommandSource, CommandTextResolver},
    sender::CommandSender,
};
use crate::server::Server;
use crate::world::World;

/// Scoped, read-only access to server text resolution during block-entity loading.
///
/// Initial disk/worldgen decoding precedes Vanilla's `setLevel` and uses
/// [`Self::detached`]. Live command loads receive an explicit context. Neither
/// block entities nor worlds retain a server reference through this API.
pub struct BlockEntityLoadContext {
    source: Option<CommandSource>,
}

impl BlockEntityLoadContext {
    /// The pre-level-binding load phase: server-dependent text is left unresolved.
    #[must_use]
    pub const fn detached() -> Self {
        Self { source: None }
    }

    pub(crate) fn for_source(source: &CommandSource) -> Result<Self, CommandSyntaxError> {
        // A live NBT load has no resolution player, even when a player ran the
        // command. Its world is the command's effective world, not the console's.
        Self::for_server_world(source.server(), source.world())
    }

    /// The live placement phase: the block entity is already attached to `world`.
    ///
    /// Mirrors `LecternBlockEntity.resolveBook` after `setLevel`: text resolves
    /// from the block center with no source entity. Playerless placement keeps
    /// [`Self::detached`], because Vanilla only resolves for a server level and
    /// Vecta has no server reference without a placing player.
    pub(crate) fn for_server_world(
        server: &Arc<Server>,
        world: &Arc<World>,
    ) -> Result<Self, CommandSyntaxError> {
        let source = CommandSource::new(CommandSender::Console, Arc::clone(server))
            .with_world(Arc::clone(world))?;
        Ok(Self {
            source: Some(source),
        })
    }

    /// Resolves written pages at the block center with no source entity.
    /// The caller must not hold inventory or block-entity container locks.
    pub fn resolve_written_book(&self, book: &mut ItemStack, pos: BlockPos) {
        let Some(source) = &self.source else { return };
        let Some(content) = book
            .get(WRITTEN_BOOK_CONTENT)
            .filter(|content| !content.resolved())
        else {
            return;
        };
        let source = source.with_position(pos.get_center().into());
        let resolver = CommandTextResolver::new(&source);
        let updated_content = content
            .resolve(&resolver)
            .unwrap_or_else(|| content.mark_resolved());
        book.set(WRITTEN_BOOK_CONTENT, updated_content);
    }
}
