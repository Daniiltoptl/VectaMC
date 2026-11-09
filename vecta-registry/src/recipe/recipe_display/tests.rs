use super::*;
use crate::recipe::vanilla_recipe_book_categories as categories;
use crate::registry::RegistryEntry as _;
use crate::vanilla_items;
use crate::{REGISTRY, init_vanilla_registry};
use std::io::Cursor;
use vecta_utils::Identifier;

fn encode(value: &impl WriteTo) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.write(&mut bytes).expect("test value should encode");
    bytes
}

fn display_prefix(kind: RecipeDisplayKind) -> Vec<u8> {
    encode(&VarInt(
        i32::try_from(kind.display_type().id()).expect("display ID fits i32"),
    ))
}

fn slot_list(displays: &[SlotDisplay]) -> Vec<u8> {
    let mut bytes = encode(&VarInt(
        i32::try_from(displays.len()).expect("fixture count fits i32"),
    ));
    for display in displays {
        bytes.extend(encode(display));
    }
    bytes
}

#[test]
fn bootstrap_order_matches_vanilla_recipe_displays() {
    init_vanilla_registry();
    // `RecipeDisplays.bootstrap` registers the five vanilla displays in this
    // order, which is the wire dispatch order for `minecraft:recipe_display`.
    for (kind, id) in [
        (RecipeDisplayKind::CraftingShapeless, 0),
        (RecipeDisplayKind::CraftingShaped, 1),
        (RecipeDisplayKind::Furnace, 2),
        (RecipeDisplayKind::Stonecutter, 3),
        (RecipeDisplayKind::Smithing, 4),
    ] {
        let registered = REGISTRY
            .recipe_displays
            .by_id(id)
            .unwrap_or_else(|| panic!("bootstrap display {id} is registered"));
        assert_eq!(registered.kind(), kind);
        assert_eq!(kind.display_type().id(), id);
    }
}

#[test]
fn display_round_trips_match_vanilla_wire_shapes() {
    init_vanilla_registry();
    let station = SlotDisplay::Item(&vanilla_items::CRAFTING_TABLE);
    let result = SlotDisplay::Item(&vanilla_items::STICK);

    let shapeless = RecipeDisplay::Shapeless {
        ingredients: vec![SlotDisplay::Item(&vanilla_items::STICK), SlotDisplay::Empty],
        result: result.clone(),
        crafting_station: station.clone(),
    };
    let mut expected = display_prefix(RecipeDisplayKind::CraftingShapeless);
    expected.extend(slot_list(&[
        SlotDisplay::Item(&vanilla_items::STICK),
        SlotDisplay::Empty,
    ]));
    expected.extend(encode(&result));
    expected.extend(encode(&station));
    round_trip(&shapeless, &expected);

    let shaped = RecipeDisplay::Shaped {
        width: 2,
        height: 1,
        ingredients: vec![SlotDisplay::Empty, SlotDisplay::Item(&vanilla_items::STICK)],
        result: result.clone(),
        crafting_station: station.clone(),
    };
    let mut expected = display_prefix(RecipeDisplayKind::CraftingShaped);
    expected.extend(encode(&VarInt(2)));
    expected.extend(encode(&VarInt(1)));
    expected.extend(slot_list(&[
        SlotDisplay::Empty,
        SlotDisplay::Item(&vanilla_items::STICK),
    ]));
    expected.extend(encode(&result));
    expected.extend(encode(&station));
    round_trip(&shaped, &expected);

    let furnace = RecipeDisplay::Furnace {
        ingredient: SlotDisplay::Item(&vanilla_items::BEEF),
        fuel: SlotDisplay::AnyFuel,
        result: SlotDisplay::Item(&vanilla_items::COOKED_BEEF),
        crafting_station: SlotDisplay::Item(&vanilla_items::FURNACE),
        duration: 200,
        experience: 0.35,
    };
    let mut expected = display_prefix(RecipeDisplayKind::Furnace);
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::BEEF)));
    expected.extend(encode(&SlotDisplay::AnyFuel));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::COOKED_BEEF)));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::FURNACE)));
    expected.extend(encode(&VarInt(200)));
    expected.extend(encode(&0.35f32));
    round_trip(&furnace, &expected);

    let stonecutter = RecipeDisplay::Stonecutter {
        input: SlotDisplay::Item(&vanilla_items::STONE),
        result: SlotDisplay::Item(&vanilla_items::STONE_BRICKS),
        crafting_station: SlotDisplay::Item(&vanilla_items::STONECUTTER),
    };
    let mut expected = display_prefix(RecipeDisplayKind::Stonecutter);
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::STONE)));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::STONE_BRICKS)));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::STONECUTTER)));
    round_trip(&stonecutter, &expected);

    let smithing = RecipeDisplay::Smithing {
        template: SlotDisplay::Empty,
        base: SlotDisplay::Item(&vanilla_items::DIAMOND_SWORD),
        addition: SlotDisplay::Item(&vanilla_items::NETHERITE_INGOT),
        result: SlotDisplay::Item(&vanilla_items::NETHERITE_SWORD),
        crafting_station: SlotDisplay::Item(&vanilla_items::SMITHING_TABLE),
    };
    let mut expected = display_prefix(RecipeDisplayKind::Smithing);
    expected.extend(encode(&SlotDisplay::Empty));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::DIAMOND_SWORD)));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::NETHERITE_INGOT)));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::NETHERITE_SWORD)));
    expected.extend(encode(&SlotDisplay::Item(&vanilla_items::SMITHING_TABLE)));
    round_trip(&smithing, &expected);
}

fn round_trip(display: &RecipeDisplay, expected: &[u8]) {
    assert_eq!(encode(display), expected, "wire bytes diverge");
    let mut cursor = Cursor::new(expected);
    let decoded = RecipeDisplay::read(&mut cursor).expect("display should decode");
    assert_eq!(decoded, *display);
    assert_eq!(cursor.position(), expected.len() as u64);
}

#[test]
fn shaped_display_rejects_mismatched_ingredient_count_on_decode() {
    init_vanilla_registry();
    // Vanilla's compact constructor: `ingredients.size() != width * height`.
    let display = RecipeDisplay::Shaped {
        width: 2,
        height: 2,
        ingredients: vec![SlotDisplay::Empty, SlotDisplay::Empty],
        result: SlotDisplay::Empty,
        crafting_station: SlotDisplay::Empty,
    };
    let bytes = encode(&display);
    assert!(RecipeDisplay::read(&mut Cursor::new(bytes.as_slice())).is_err());

    let valid = RecipeDisplay::Shaped {
        width: 2,
        height: 2,
        ingredients: vec![SlotDisplay::Empty; 4],
        result: SlotDisplay::Empty,
        crafting_station: SlotDisplay::Empty,
    };
    let bytes = encode(&valid);
    let decoded =
        RecipeDisplay::read(&mut Cursor::new(bytes.as_slice())).expect("matching grid decodes");
    assert_eq!(decoded, valid);
}

#[test]
fn entry_round_trips_group_category_and_requirements() {
    init_vanilla_registry();
    let display = RecipeDisplay::Shapeless {
        ingredients: vec![SlotDisplay::Empty],
        result: SlotDisplay::Empty,
        crafting_station: SlotDisplay::Empty,
    };

    let entry = RecipeDisplayEntry {
        id: RecipeDisplayId { index: 42 },
        display: display.clone(),
        group: None,
        category: &categories::CRAFTING_MISC,
        crafting_requirements: None,
    };
    let mut expected = encode(&RecipeDisplayId { index: 42 });
    expected.extend(encode(&display));
    expected.extend(encode(&VarInt(0))); // OPTIONAL_VAR_INT: absent.
    expected.extend(encode(&VarInt(
        i32::try_from(categories::CRAFTING_MISC.id()).expect("category ID fits i32"),
    )));
    expected.push(0); // Requirements absent.
    assert_eq!(encode(&entry), expected);
    read_back(&entry, &expected);

    let entry = RecipeDisplayEntry {
        group: Some(3),
        crafting_requirements: Some(vec![RegistryHolderSet::Direct(vec![&vanilla_items::STICK])]),
        ..entry
    };
    let mut expected = encode(&RecipeDisplayId { index: 42 });
    expected.extend(encode(&display));
    expected.extend(encode(&VarInt(4))); // OPTIONAL_VAR_INT: value + 1.
    expected.extend(encode(&VarInt(
        i32::try_from(categories::CRAFTING_MISC.id()).expect("category ID fits i32"),
    )));
    expected.push(1); // Requirements present.
    // `ByteBufCodecs.list()`: VarInt count of requirement sets, then each
    // holder set (direct set writes count + 1, then item IDs).
    expected.extend(encode(&VarInt(1)));
    expected.extend(encode(&VarInt(2)));
    expected.extend(encode(&VarInt(
        i32::try_from(vanilla_items::STICK.id()).expect("item ID fits i32"),
    )));
    assert_eq!(encode(&entry), expected);
    read_back(&entry, &expected);
}

fn read_back(entry: &RecipeDisplayEntry, expected: &[u8]) {
    let mut cursor = Cursor::new(expected);
    let decoded = RecipeDisplayEntry::read(&mut cursor).expect("entry should decode");
    assert_eq!(&decoded, entry);
    assert_eq!(cursor.position(), expected.len() as u64);
}

#[test]
fn entry_decoding_rejects_unknown_ids() {
    init_vanilla_registry();
    let entry = RecipeDisplayEntry {
        id: RecipeDisplayId { index: 0 },
        display: RecipeDisplay::Stonecutter {
            input: SlotDisplay::Empty,
            result: SlotDisplay::Empty,
            crafting_station: SlotDisplay::Empty,
        },
        group: None,
        category: &categories::STONECUTTER,
        crafting_requirements: None,
    };
    let bytes = encode(&entry);

    // Wire layout: recipe id, display id, payload..., group, category,
    // requirements flag. Corrupt the display discriminator first.
    let mut unknown_display = bytes.clone();
    unknown_display[1] = 200; // VarInt(200) = unregistered display id.
    assert!(RecipeDisplayEntry::read(&mut Cursor::new(unknown_display.as_slice())).is_err());

    // Corrupt the category id (second-to-last byte) to an unregistered id.
    let mut unknown_category = bytes.clone();
    let last = unknown_category.len() - 2;
    unknown_category[last] = 200;
    assert!(RecipeDisplayEntry::read(&mut Cursor::new(unknown_category.as_slice())).is_err());

    // A truncated recipe display id fails outright.
    let mut truncated_id = bytes;
    truncated_id[0] = 0x80; // Continuation bit with no follow-up byte.
    assert!(RecipeDisplayEntry::read(&mut Cursor::new(truncated_id.as_slice())).is_err());
}

#[test]
fn generated_registry_order_matches_the_versioned_extractor_output() {
    init_vanilla_registry();
    let extracted: serde_json::Value =
        serde_json::from_str(include_str!("../../../build_assets/recipe_displays.json"))
            .expect("extractor output is valid JSON");
    let entries = extracted["entries"]
        .as_array()
        .expect("entries is an array");
    assert_eq!(entries.len(), REGISTRY.recipe_displays.iter().count());
    for entry in entries {
        let id = usize::try_from(entry["id"].as_u64().expect("extracted ID is unsigned"))
            .expect("ID fits usize");
        let registered = REGISTRY
            .recipe_displays
            .by_id(id)
            .expect("extracted entry is registered");
        assert_eq!(
            registered.key.to_string(),
            entry["key"].as_str().expect("key is a string")
        );
        assert_eq!(registered.id(), id);
    }
}

#[test]
fn local_registries_encode_and_decode_without_the_global_registry() {
    init_vanilla_registry();
    static LOCAL: RecipeDisplayType = RecipeDisplayType::vanilla(
        Identifier::new_static("vecta", "test_recipe_display"),
        RecipeDisplayKind::Furnace,
    );
    let mut registry = RecipeDisplayTypeRegistry::new();
    registry.register(&crate::vanilla_recipe_displays::CRAFTING_SHAPELESS);
    let id = registry.register(&LOCAL);
    registry.freeze();
    assert_eq!(
        registry.registered_entry_with_id(&LOCAL),
        Some((id, &LOCAL))
    );

    let display = RecipeDisplay::Furnace {
        ingredient: SlotDisplay::Empty,
        fuel: SlotDisplay::Empty,
        result: SlotDisplay::Empty,
        crafting_station: SlotDisplay::Empty,
        duration: 100,
        experience: 0.0,
    };
    LOCAL
        .write_payload(&display, &mut Vec::new())
        .expect("local payload should encode");

    // A vanilla display type refuses a mismatched payload kind.
    let shapeless = RecipeDisplay::Shapeless {
        ingredients: Vec::new(),
        result: SlotDisplay::Empty,
        crafting_station: SlotDisplay::Empty,
    };
    assert!(LOCAL.write_payload(&shapeless, &mut Vec::new()).is_err());

    // Kind-based dispatch always resolves to the globally registered statics,
    // so vanilla-kinded displays encode against the global wire registry even
    // while a same-kind local type exists.
    let bytes = encode(&display);
    let mut cursor = Cursor::new(bytes.as_slice());
    let discriminator = VarInt::read(&mut cursor)
        .expect("display id should decode")
        .0;
    assert_eq!(
        usize::try_from(discriminator).expect("display id is nonnegative"),
        crate::vanilla_recipe_displays::FURNACE.id()
    );
    let decoded = RecipeDisplay::read(&mut Cursor::new(bytes.as_slice()))
        .expect("global payload should decode");
    assert_eq!(decoded, display);
}

#[test]
fn group_encoding_rejects_negative_values() {
    init_vanilla_registry();
    let entry = RecipeDisplayEntry {
        id: RecipeDisplayId { index: 0 },
        display: RecipeDisplay::Stonecutter {
            input: SlotDisplay::Empty,
            result: SlotDisplay::Empty,
            crafting_station: SlotDisplay::Empty,
        },
        group: Some(-1),
        category: &categories::STONECUTTER,
        crafting_requirements: None,
    };
    assert!(
        entry.write(&mut Vec::new()).is_err(),
        "a negative group cannot be encoded as OPTIONAL_VAR_INT"
    );
}
