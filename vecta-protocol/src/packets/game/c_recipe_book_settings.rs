//! Clientbound packet that syncs the recipe book tab settings.

use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_RECIPE_BOOK_SETTINGS;
use vecta_registry::recipe::RecipeBookSettings;

/// Vanilla `ClientboundRecipeBookSettingsPacket`.
///
/// Eight booleans in vanilla `RecipeBookSettings.STREAM_CODEC` order:
/// crafting, furnace, blast furnace, smoker — each open/filtering pair.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_RECIPE_BOOK_SETTINGS)]
pub struct CRecipeBookSettings {
    /// Vanilla `bookSettings` (`RecipeBookSettings.STREAM_CODEC`).
    pub book_settings: RecipeBookSettings,
}

#[cfg(test)]
mod tests {
    use vecta_registry::recipe::{RecipeBookSettings, RecipeBookType};
    use vecta_utils::serial::WriteTo as _;

    use super::CRecipeBookSettings;

    #[test]
    fn writes_the_eight_tab_bools_in_vanilla_order() {
        let mut settings = RecipeBookSettings::default();
        settings.set_open(RecipeBookType::Crafting, true);
        settings.set_filtering(RecipeBookType::Smoker, true);

        let packet = CRecipeBookSettings {
            book_settings: settings,
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).expect("packet should encode");
        assert_eq!(bytes, vec![1, 0, 0, 0, 0, 0, 0, 1]);
    }
}
