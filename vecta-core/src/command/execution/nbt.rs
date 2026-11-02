//! NBT path and SNBT command arguments.

use simdnbt::owned::{NbtCompound, NbtTag};
use text_components::TextComponent;
use vecta_utils::nbt::{
    NbtPath, parse_nbt_path_argument as parse_path, parse_snbt_argument,
    parse_snbt_compound_argument,
};

use crate::command::brigadier::{CommandSyntaxError, CommandSyntaxErrorKind, StringReader};

pub(super) fn parse_nbt_path(reader: &mut StringReader<'_>) -> Result<NbtPath, CommandSyntaxError> {
    match parse_path(reader.remaining()) {
        Ok((path, consumed)) => {
            if !reader.advance_bytes(consumed) {
                return Err(dynamic_error(reader, "Invalid NBT path cursor"));
            }
            Ok(path)
        }
        Err(error) => {
            if !reader.advance_bytes(error.cursor()) {
                return Err(dynamic_error(reader, "Invalid NBT path cursor"));
            }
            Err(dynamic_error(reader, error.component()))
        }
    }
}

/// Parses a `{}`-delimited SNBT compound, like vanilla's `CompoundTagArgument`.
pub(super) fn parse_compound_tag(
    reader: &mut StringReader<'_>,
) -> Result<NbtCompound, CommandSyntaxError> {
    match parse_snbt_compound_argument(reader.remaining()) {
        Ok((compound, consumed)) => {
            if !reader.advance_bytes(consumed) {
                return Err(dynamic_error(reader, "Invalid NBT cursor"));
            }
            Ok(compound)
        }
        Err(error) => {
            if !reader.advance_bytes(error.cursor()) {
                return Err(dynamic_error(reader, "Invalid NBT cursor"));
            }
            Err(dynamic_error(reader, error.component()))
        }
    }
}

/// Parses any single SNBT tag, like vanilla's `NbtTagArgument`.
pub(super) fn parse_nbt_tag(reader: &mut StringReader<'_>) -> Result<NbtTag, CommandSyntaxError> {
    match parse_snbt_argument(reader.remaining()) {
        Ok((tag, consumed)) => {
            if !reader.advance_bytes(consumed) {
                return Err(dynamic_error(reader, "Invalid NBT cursor"));
            }
            Ok(tag)
        }
        Err(error) => {
            if !reader.advance_bytes(error.cursor()) {
                return Err(dynamic_error(reader, "Invalid NBT cursor"));
            }
            Err(dynamic_error(reader, error.component()))
        }
    }
}

fn dynamic_error(
    reader: &StringReader<'_>,
    message: impl Into<TextComponent>,
) -> CommandSyntaxError {
    reader.error(CommandSyntaxErrorKind::Dynamic(Box::new(message.into())))
}
