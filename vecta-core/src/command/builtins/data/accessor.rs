//! `/data` targets and the live-update rules that follow a write.

use std::sync::Arc;

use simdnbt::owned::{NbtCompound, NbtTag};
use text_components::TextComponent;
use vecta_utils::{BlockPos, Identifier, nbt::to_canonical_snbt, translations};

use super::super::super::{
    brigadier::CommandSyntaxError,
    execution::{CommandSource, VectaArgumentType, VectaCommandContext},
};
use super::super::execute::{loaded_block_position, source_command_storage};
use crate::block_entity::{BlockEntityLoadContext, SharedBlockEntity};

/// A vanilla `/data` target provider.
#[derive(Clone, Copy, Debug)]
pub(super) enum Provider {
    Block,
    Storage,
}

impl Provider {
    /// Target and source providers in vanilla registration order.
    pub(super) const ALL: [Self; 2] = [Self::Block, Self::Storage];

    /// The literal that introduces this provider in the command graph.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Storage => "storage",
        }
    }

    pub(super) fn argument_type(self) -> VectaArgumentType {
        match self {
            Self::Block => VectaArgumentType::block_pos(),
            Self::Storage => VectaArgumentType::storage_key(),
        }
    }

    /// Resolves the provider's argument into a usable accessor.
    pub(super) fn resolve(
        self,
        context: &VectaCommandContext<CommandSource>,
        name: &str,
    ) -> Result<DataAccessor, CommandSyntaxError> {
        match self {
            Self::Block => {
                let pos = loaded_block_position(context, name)?;
                let block_entity =
                    context
                        .source()
                        .world()
                        .get_block_entity(pos)
                        .ok_or_else(|| {
                            CommandSyntaxError::dynamic(TextComponent::from(
                                &translations::COMMANDS_DATA_BLOCK_INVALID,
                            ))
                        })?;
                Ok(DataAccessor {
                    target: Target::Block { pos, block_entity },
                })
            }
            Self::Storage => Ok(DataAccessor {
                target: Target::Storage(context.identifier(name)?.clone()),
            }),
        }
    }
}

/// Read/write access to one `/data` target.
pub(super) struct DataAccessor {
    target: Target,
}

enum Target {
    Block {
        pos: BlockPos,
        block_entity: SharedBlockEntity,
    },
    Storage(Identifier),
}

impl DataAccessor {
    /// Returns the target's command-visible compound.
    pub(super) fn get_data(
        &self,
        source: &CommandSource,
    ) -> Result<NbtCompound, CommandSyntaxError> {
        match &self.target {
            Target::Block { block_entity, .. } => Ok(block_entity.save_with_full_metadata()),
            Target::Storage(id) => Ok(source_command_storage(source)?.get(id)),
        }
    }

    /// Writes a compound back into the target.
    ///
    /// A block entity is reloaded through the command's live read context, so
    /// server-dependent text resolves the same way it does for `/setblock` and
    /// `/execute store`. A rejected payload restores the previous state instead
    /// of leaving a half-applied entity behind.
    pub(super) fn set_data(
        &self,
        source: &CommandSource,
        data: NbtCompound,
    ) -> Result<(), CommandSyntaxError> {
        match &self.target {
            Target::Block { pos, block_entity } => {
                let world = Arc::clone(source.world());
                let load_context = BlockEntityLoadContext::for_source(source)?;
                let snapshot = block_entity.save_with_full_metadata();
                if let Err(error) = block_entity.load_with_owned_components(&data, &load_context) {
                    block_entity
                        .load_with_owned_components(&snapshot, &BlockEntityLoadContext::detached())
                        .ok();
                    return Err(CommandSyntaxError::dynamic(format!(
                        "Failed to load block entity data at {pos:?}: {error}"
                    )));
                }

                // Vanilla `BlockDataAccessor.setData` marks the entity changed and
                // broadcasts the new block-entity data; `setChanged` also refreshes
                // the comparator output and the chunk's unsaved flag.
                block_entity.set_changed();
                world.send_block_updated(*pos);
                Ok(())
            }
            Target::Storage(id) => {
                source_command_storage(source)?.set(id.clone(), data);
                Ok(())
            }
        }
    }

    pub(super) fn modified_success(&self) -> TextComponent {
        match &self.target {
            Target::Block { pos, .. } => translations::COMMANDS_DATA_BLOCK_MODIFIED
                .message([
                    pos.x().to_string(),
                    pos.y().to_string(),
                    pos.z().to_string(),
                ])
                .component(),
            Target::Storage(id) => translations::COMMANDS_DATA_STORAGE_MODIFIED
                .message([id.to_string()])
                .component(),
        }
    }

    pub(super) fn print_success(&self, tag: &NbtTag) -> TextComponent {
        let printed = TextComponent::plain(render_tag(tag));
        match &self.target {
            Target::Block { pos, .. } => translations::COMMANDS_DATA_BLOCK_QUERY
                .message([
                    TextComponent::plain(pos.x().to_string()),
                    TextComponent::plain(pos.y().to_string()),
                    TextComponent::plain(pos.z().to_string()),
                    printed,
                ])
                .component(),
            Target::Storage(id) => translations::COMMANDS_DATA_STORAGE_QUERY
                .message([TextComponent::plain(id.to_string()), printed])
                .component(),
        }
    }

    pub(super) fn print_success_scaled(&self, path: &str, scale: f64, value: i32) -> TextComponent {
        let scale = TextComponent::plain(format!("{scale:.2}"));
        let value = TextComponent::plain(value.to_string());
        match &self.target {
            Target::Block { pos, .. } => translations::COMMANDS_DATA_BLOCK_GET
                .message([
                    TextComponent::plain(path.to_owned()),
                    TextComponent::plain(pos.x().to_string()),
                    TextComponent::plain(pos.y().to_string()),
                    TextComponent::plain(pos.z().to_string()),
                    scale,
                    value,
                ])
                .component(),
            Target::Storage(id) => translations::COMMANDS_DATA_STORAGE_GET
                .message([
                    TextComponent::plain(path.to_owned()),
                    TextComponent::plain(id.to_string()),
                    scale,
                    value,
                ])
                .component(),
        }
    }
}

/// Renders a tag for command output.
///
/// Vanilla's `NbtUtils.toPrettyComponent` colours keys and values; Vecta prints
/// canonical SNBT as plain text, which keeps the same readable content without a
/// dedicated NBT styling renderer.
pub(super) fn render_tag(tag: &NbtTag) -> String {
    to_canonical_snbt(tag).unwrap_or_else(|| format!("{tag:?}"))
}
