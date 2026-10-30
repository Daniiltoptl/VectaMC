use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_UPDATE_RECIPES;
use vecta_registry::recipe::{RecipePropertySet, SingleInputEntry};
use vecta_utils::Identifier;

/// Vanilla `ClientboundUpdateRecipesPacket`.
///
/// Sent on login and after a datapack reload. `item_sets` feeds the recipe book
/// filters and `stonecutter_recipes` is the list the stonecutter screen filters
/// by the held input, so both must arrive before the client opens a menu.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_UPDATE_RECIPES)]
pub struct CUpdateRecipes {
    /// `Map<ResourceKey<RecipePropertySet>, RecipePropertySet>`.
    pub item_sets: Vec<(Identifier, RecipePropertySet)>,
    /// `SelectableRecipe.SingleInputSet<StonecutterRecipe>`.
    pub stonecutter_recipes: Vec<SingleInputEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use vecta_registry::recipe::{Ingredient, SlotDisplay};
    use vecta_registry::{ItemStackTemplate, RegistryEntry as _, vanilla_items};
    use vecta_utils::serial::WriteTo as _;

    /// Item ids only exist once the vanilla registry is published.
    fn registry() {
        vecta_registry::init_vanilla_registry();
    }

    fn encode(packet: &CUpdateRecipes) -> Vec<u8> {
        let mut bytes = Vec::new();
        packet.write(&mut bytes).expect("packet should encode");
        bytes
    }

    fn identifier_bytes(value: &Identifier) -> Vec<u8> {
        let mut bytes = Vec::new();
        value.write(&mut bytes).expect("identifier should encode");
        bytes
    }

    fn empty_packet() -> CUpdateRecipes {
        CUpdateRecipes {
            item_sets: Vec::new(),
            stonecutter_recipes: Vec::new(),
        }
    }

    #[test]
    fn empty_packet_writes_two_zero_counts() {
        // `ByteBufCodecs.map` and `ByteBufCodecs.list` both start with a VarInt size.
        assert_eq!(encode(&empty_packet()), vec![0, 0]);
    }

    #[test]
    fn item_set_entry_writes_key_then_item_ids() {
        registry();
        let key = Identifier::new("minecraft", "smithing_base");
        let mut packet = empty_packet();
        packet.item_sets.push((
            key.clone(),
            RecipePropertySet::from_ingredients([&Ingredient::Item(&vanilla_items::STONE)]),
        ));

        let mut expected = vec![1];
        expected.extend(identifier_bytes(&key));
        expected.push(1);
        expected.push(u8::try_from(vanilla_items::STONE.id()).expect("stone id fits a byte"));
        expected.push(0);

        assert_eq!(encode(&packet), expected);
    }

    #[test]
    fn stonecutter_entry_writes_tag_ingredient_then_item_stack_display() {
        registry();
        let tag = Identifier::new("minecraft", "stone_tool_materials");
        let result = ItemStackTemplate::new(&vanilla_items::STONECUTTER);
        let mut packet = empty_packet();
        packet.stonecutter_recipes.push(SingleInputEntry {
            input: Ingredient::Tag(tag.clone()),
            option_display: SlotDisplay::ItemStack(result.clone()),
        });

        // `ByteBufCodecs.holderSet` writes `0` for a named set, then the tag id;
        // `SlotDisplay.STREAM_CODEC` writes the `item_stack` kind, then the template.
        let mut expected = vec![0, 1, 0];
        expected.extend(identifier_bytes(&tag));
        expected.push(5);
        let mut template = Vec::new();
        result.write(&mut template).expect("template should encode");
        expected.extend(template);

        assert_eq!(encode(&packet), expected);
    }

    #[test]
    fn composite_display_writes_each_element() {
        registry();
        let display = SlotDisplay::Composite(vec![
            SlotDisplay::Empty,
            SlotDisplay::Item(&vanilla_items::STONE),
        ]);
        let mut bytes = Vec::new();
        display.write(&mut bytes).expect("display should encode");
        let mut expected = vec![10, 2, 0, 4];
        expected.push(u8::try_from(vanilla_items::STONE.id()).expect("stone id fits a byte"));
        assert_eq!(bytes, expected);
    }
}
