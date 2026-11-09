//! Serverbound packet when the client marks a recipe book entry as seen.

use vecta_macros::{ReadFrom, ServerPacket};
use vecta_registry::recipe::RecipeDisplayId;

/// Vanilla `ServerboundRecipeBookSeenRecipePacket`.
///
/// The client sends it once the player opened the new-recipe notification;
/// the server clears that recipe's highlight so it stops being shown.
#[derive(ReadFrom, ServerPacket, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SRecipeBookSeenRecipe {
    /// The display id the client is acknowledging.
    pub recipe: RecipeDisplayId,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use vecta_registry::recipe::RecipeDisplayId;
    use vecta_utils::codec::VarInt;
    use vecta_utils::serial::{ReadFrom as _, WriteTo as _};

    use super::SRecipeBookSeenRecipe;

    #[test]
    fn round_trips_through_a_varint_index() {
        let packet = SRecipeBookSeenRecipe {
            recipe: RecipeDisplayId { index: 300 },
        };

        let mut bytes = Vec::new();
        packet
            .recipe
            .write(&mut bytes)
            .expect("seen recipe packet should encode");
        let mut expected = Vec::new();
        VarInt(300)
            .write(&mut expected)
            .expect("varint should encode");
        assert_eq!(
            bytes, expected,
            "`RecipeDisplayId.STREAM_CODEC` is a VarInt"
        );

        let decoded = SRecipeBookSeenRecipe::read(&mut Cursor::new(bytes.as_slice()))
            .expect("seen recipe packet should decode");
        assert_eq!(decoded, packet);
    }
}
