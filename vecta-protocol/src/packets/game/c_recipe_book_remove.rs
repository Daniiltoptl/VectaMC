//! Clientbound packet that removes recipe book entries on the client.

use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_RECIPE_BOOK_REMOVE;
use vecta_registry::recipe::RecipeDisplayId;

/// Vanilla `ClientboundRecipeBookRemovePacket`.
///
/// The ids are display ids resolved through the same display index that
/// produced the add packets, so the client can drop the exact entries.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_RECIPE_BOOK_REMOVE)]
pub struct CRecipeBookRemove {
    /// Display ids of the recipes to forget.
    pub recipes: Vec<RecipeDisplayId>,
}

#[cfg(test)]
mod tests {
    use vecta_registry::recipe::RecipeDisplayId;
    use vecta_utils::codec::VarInt;
    use vecta_utils::serial::WriteTo as _;

    use super::CRecipeBookRemove;

    #[test]
    fn writes_a_varint_count_then_display_ids() {
        let packet = CRecipeBookRemove {
            recipes: vec![RecipeDisplayId { index: 0 }, RecipeDisplayId { index: 300 }],
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).expect("packet should encode");

        let mut expected = vec![2];
        VarInt(0).write(&mut expected).expect("id 0 should encode");
        VarInt(300)
            .write(&mut expected)
            .expect("id 300 should encode");
        assert_eq!(bytes, expected);
    }

    #[test]
    fn empty_packet_writes_a_single_zero() {
        let packet = CRecipeBookRemove {
            recipes: Vec::new(),
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).expect("packet should encode");
        assert_eq!(bytes, vec![0]);
    }
}
