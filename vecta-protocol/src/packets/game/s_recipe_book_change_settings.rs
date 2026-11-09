//! Serverbound packet when the player changes recipe book tab settings.

use std::io::{Cursor, Error, ErrorKind, Result, Write};

use vecta_macros::ServerPacket;
use vecta_registry::recipe::RecipeBookType;
use vecta_utils::codec::VarInt;
use vecta_utils::serial::{ReadFrom, WriteTo};

/// Vanilla `ServerboundRecipeBookChangeSettingsPacket`.
///
/// The client reports the open/filtering state of one recipe book tab; the
/// server stores it so the next login restores the tabs
/// (`ClientboundRecipeBookSettingsPacket` in `sendInitialRecipeBook`).
#[derive(ServerPacket, Debug, Clone, PartialEq, Eq)]
pub struct SRecipeBookChangeSettings {
    /// Vanilla `readEnum(RecipeBookType.class)`: `VarInt` ordinal.
    pub book_type: RecipeBookType,
    pub is_open: bool,
    pub is_filtering: bool,
}

/// Vanilla `writeEnum`: the `VarInt` ordinal of `RecipeBookType`
/// (`CRAFTING`, `FURNACE`, `BLAST_FURNACE`, `SMOKER`).
const fn book_type_ordinal(book_type: RecipeBookType) -> i32 {
    match book_type {
        RecipeBookType::Crafting => 0,
        RecipeBookType::Furnace => 1,
        RecipeBookType::BlastFurnace => 2,
        RecipeBookType::Smoker => 3,
    }
}

/// Inverse of `book_type_ordinal` with vanilla's out-of-range error.
fn book_type_from_ordinal(ordinal: i32) -> Result<RecipeBookType> {
    match ordinal {
        0 => Ok(RecipeBookType::Crafting),
        1 => Ok(RecipeBookType::Furnace),
        2 => Ok(RecipeBookType::BlastFurnace),
        3 => Ok(RecipeBookType::Smoker),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("unknown recipe book type ordinal: {ordinal}"),
        )),
    }
}

impl WriteTo for SRecipeBookChangeSettings {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        VarInt(book_type_ordinal(self.book_type)).write(writer)?;
        self.is_open.write(writer)?;
        self.is_filtering.write(writer)
    }
}

impl ReadFrom for SRecipeBookChangeSettings {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let ordinal = VarInt::read(data)?.0;
        Ok(Self {
            book_type: book_type_from_ordinal(ordinal)?,
            is_open: bool::read(data)?,
            is_filtering: bool::read(data)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use vecta_utils::serial::{ReadFrom as _, WriteTo as _};

    use super::{RecipeBookType, SRecipeBookChangeSettings};

    fn round_trip(packet: &SRecipeBookChangeSettings) -> SRecipeBookChangeSettings {
        let mut bytes = Vec::new();
        packet
            .write(&mut bytes)
            .expect("recipe book settings packet should encode");
        SRecipeBookChangeSettings::read(&mut Cursor::new(bytes.as_slice()))
            .expect("recipe book settings packet should decode")
    }

    #[test]
    fn round_trips_every_book_type() {
        for (ordinal, book_type) in [
            (0, RecipeBookType::Crafting),
            (1, RecipeBookType::Furnace),
            (2, RecipeBookType::BlastFurnace),
            (3, RecipeBookType::Smoker),
        ] {
            let packet = SRecipeBookChangeSettings {
                book_type,
                is_open: true,
                is_filtering: ordinal % 2 == 1,
            };
            let mut bytes = Vec::new();
            packet
                .write(&mut bytes)
                .expect("recipe book settings packet should encode");
            assert_eq!(
                bytes,
                vec![ordinal, 1, u8::from(ordinal % 2 == 1)],
                "vanilla writes the tab as a VarInt ordinal, then two bools"
            );
            assert_eq!(round_trip(&packet), packet);
        }
    }

    #[test]
    fn rejects_unknown_ordinals() {
        let error = SRecipeBookChangeSettings::read(&mut Cursor::new(&[4, 0, 0]))
            .expect_err("ordinal 4 is outside the vanilla RecipeBookType range");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
