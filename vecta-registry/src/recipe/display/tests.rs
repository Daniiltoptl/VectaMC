use super::*;
use crate::trim_pattern::TrimPatternValue;
use crate::vanilla_item_tags::ItemTag;
use crate::{init_vanilla_registry, vanilla_items, vanilla_trim_patterns};
use text_components::TextComponent;

fn encode(value: &impl WriteTo) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.write(&mut bytes).expect("test value should encode");
    bytes
}

fn display_prefix(kind: SlotDisplayKind) -> Vec<u8> {
    encode(&VarInt(
        i32::try_from(kind.display_type().id()).expect("display ID fits i32"),
    ))
}

fn round_trip(display: &SlotDisplay, expected: &[u8]) {
    assert_eq!(encode(display), expected);
    let mut cursor = Cursor::new(expected);
    assert_eq!(
        SlotDisplay::read(&mut cursor).expect("display should decode"),
        *display
    );
    assert_eq!(cursor.position(), expected.len() as u64);
}

#[test]
fn trim_display_distinguishes_reference_from_inline_pattern() {
    init_vanilla_registry();
    let reference = &vanilla_trim_patterns::SENTRY;
    let inline = TrimPatternValue::new(
        Identifier::new_static("vecta", "inline_trim"),
        TextComponent::plain("Inline pattern"),
        true,
    );
    for pattern in [
        RegistryHolder::reference(reference),
        RegistryHolder::direct(inline),
    ] {
        let mut expected = display_prefix(SlotDisplayKind::SmithingTrim);
        // Two nested empty displays precede the holder. Registry holders shift
        // the reference id; inline holders write zero and the complete value.
        expected.extend(encode(&SlotDisplay::Empty));
        expected.extend(encode(&SlotDisplay::Empty));
        match &pattern {
            RegistryHolder::Reference(entry) => {
                expected.extend(encode(&VarInt(
                    i32::try_from(entry.id()).expect("pattern ID fits i32") + 1,
                )));
            }
            RegistryHolder::Direct(value) => {
                expected.push(0);
                expected.extend(encode(value.asset_id()));
                expected.extend(encode(&value.description().to_codec_nbt()));
                expected.extend(encode(&value.decal()));
            }
        }
        round_trip(
            &SlotDisplay::SmithingTrim {
                base: Box::new(SlotDisplay::Empty),
                material: Box::new(SlotDisplay::Empty),
                pattern: Box::new(pattern),
            },
            &expected,
        );
    }
}

#[test]
fn tag_display_accepts_named_empty_singleton_and_multiple_item_sets() {
    init_vanilla_registry();
    for items in [
        None::<Vec<ItemRef>>,
        Some(Vec::new()),
        Some(vec![&vanilla_items::STICK]),
        Some(vec![&vanilla_items::DIAMOND, &vanilla_items::STICK]),
    ] {
        let mut expected = display_prefix(SlotDisplayKind::Tag);
        let set = match items {
            None => {
                expected.push(0);
                expected.extend(encode(&ItemTag::WOOL));
                RegistryHolderSet::Tag(ItemTag::WOOL)
            }
            Some(items) => {
                expected.extend(encode(&VarInt(
                    i32::try_from(items.len()).expect("fixture count fits i32") + 1,
                )));
                for item in &items {
                    expected.extend(encode(&VarInt(
                        i32::try_from(item.id()).expect("item ID fits i32"),
                    )));
                }
                RegistryHolderSet::Direct(items)
            }
        };
        round_trip(&SlotDisplay::Tag(set), &expected);
    }
}

#[test]
fn selectable_recipe_decodes_multiple_items_and_nested_result_without_recipe_payload() {
    static CHOICES: std::sync::LazyLock<[ItemRef; 2]> =
        std::sync::LazyLock::new(|| [&vanilla_items::STONE, &vanilla_items::COBBLESTONE]);
    init_vanilla_registry();
    let display = SlotDisplay::WithRemainder {
        input: Box::new(SlotDisplay::Composite(vec![
            SlotDisplay::Empty,
            SlotDisplay::Item(&vanilla_items::STONE),
        ])),
        remainder: Box::new(SlotDisplay::AnyFuel),
    };
    let entry = SingleInputEntry::from_ingredient(&Ingredient::Choice(&*CHOICES), display);
    let mut expected = vec![3]; // Direct holder set: item count plus one.
    for item in *CHOICES {
        expected.extend(encode(&VarInt(
            i32::try_from(item.id()).expect("item ID fits i32"),
        )));
    }
    expected.extend(encode(&entry.option_display));
    assert_eq!(encode(&entry), expected);

    let mut cursor = Cursor::new(expected.as_slice());
    let decoded = SingleInputEntry::read(&mut cursor).expect("multi-item entry should decode");
    assert_eq!(decoded.input.entries(), *CHOICES);
    assert_eq!(decoded.option_display, entry.option_display);
    assert_eq!(cursor.position(), expected.len() as u64);
}

#[test]
fn selectable_recipe_preserves_empty_singleton_and_named_ingredients() {
    init_vanilla_registry();
    for ingredient in [
        Ingredient::Empty,
        Ingredient::Item(&vanilla_items::STICK),
        Ingredient::Tag(ItemTag::WOOL),
    ] {
        let entry = SingleInputEntry::from_ingredient(&ingredient, SlotDisplay::Empty);
        let bytes = encode(&entry);
        let mut cursor = Cursor::new(bytes.as_slice());
        let decoded = SingleInputEntry::read(&mut cursor).expect("entry should decode");
        assert_eq!(decoded.input, entry.input);
        assert_eq!(decoded.input.entries(), ingredient.get_items());
        assert_eq!(cursor.position(), bytes.len() as u64);
    }
}

#[test]
fn malformed_holder_payloads_fail_instead_of_decoding_a_different_display() {
    init_vanilla_registry();
    for invalid_id in [-1, i32::MAX] {
        let mut trim = display_prefix(SlotDisplayKind::SmithingTrim);
        trim.extend(encode(&SlotDisplay::Empty));
        trim.extend(encode(&SlotDisplay::Empty));
        trim.extend(encode(&VarInt(invalid_id)));
        assert!(SlotDisplay::read(&mut Cursor::new(trim.as_slice())).is_err());

        let mut tag = display_prefix(SlotDisplayKind::Tag);
        tag.push(2); // A direct set with one invalid registry reference.
        tag.extend(encode(&VarInt(invalid_id)));
        assert!(SlotDisplay::read(&mut Cursor::new(tag.as_slice())).is_err());
    }
    let mut negative_count = display_prefix(SlotDisplayKind::Tag);
    negative_count.extend(encode(&VarInt(-1)));
    assert!(SlotDisplay::read(&mut Cursor::new(negative_count.as_slice())).is_err());

    let mut truncated_inline = display_prefix(SlotDisplayKind::SmithingTrim);
    truncated_inline.extend(encode(&SlotDisplay::Empty));
    truncated_inline.extend(encode(&SlotDisplay::Empty));
    truncated_inline.push(0);
    assert!(SlotDisplay::read(&mut Cursor::new(truncated_inline.as_slice())).is_err());
}

#[test]
fn generated_registry_order_matches_the_versioned_extractor_output() {
    init_vanilla_registry();
    let extracted: serde_json::Value =
        serde_json::from_str(include_str!("../../../build_assets/slot_displays.json"))
            .expect("extractor output is valid JSON");
    let entries = extracted["entries"]
        .as_array()
        .expect("entries is an array");
    assert_eq!(entries.len(), REGISTRY.slot_displays.iter().count());
    for entry in entries {
        let id = usize::try_from(entry["id"].as_u64().expect("extracted ID is unsigned"))
            .expect("ID fits usize");
        let registered = REGISTRY
            .slot_displays
            .by_id(id)
            .expect("extracted entry is registered");
        assert_eq!(
            registered.key.to_string(),
            entry["key"].as_str().expect("key is a string")
        );
        assert_eq!(registered.id(), id);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestDisplayPayload(i32);

// SAFETY: This key uniquely identifies this Vecta-owned test implementation.
unsafe impl vecta_utils::DowncastType for TestDisplayPayload {
    const TYPE_KEY: vecta_utils::DowncastTypeKey =
        vecta_utils::DowncastTypeKey::new("vecta:test/slot_display_payload");
}

impl SlotDisplayCodec for TestDisplayPayload {
    fn read_network(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self(VarInt::read(data)?.0))
    }

    fn write_network(&self, writer: &mut Vec<u8>) -> Result<()> {
        VarInt(self.0).write(writer)
    }
}

#[test]
fn custom_payload_dispatch_checks_ownership_and_does_not_assume_a_vanilla_id() {
    static CUSTOM: SlotDisplayType =
        SlotDisplayType::of::<TestDisplayPayload>(Identifier::new_static("vecta", "test_display"));
    static IMPOSTOR: SlotDisplayType =
        SlotDisplayType::of::<TestDisplayPayload>(Identifier::new_static("vecta", "test_display"));
    init_vanilla_registry();
    let mut registry = SlotDisplayTypeRegistry::new();
    registry.register(&crate::vanilla_slot_displays::ANY_FUEL);
    registry.register(&CUSTOM);
    registry.freeze();
    assert!(registry.registered_entry_with_id(&IMPOSTOR).is_none());
    let (id, display_type) = registry
        .registered_entry_with_id(&CUSTOM)
        .expect("custom type is registered locally");
    let display = SlotDisplay::Custom(
        CustomSlotDisplay::new(&CUSTOM, TestDisplayPayload(400)).expect("matching payload type"),
    );
    let mut bytes = encode(&VarInt(i32::try_from(id).expect("local ID fits i32")));
    display_type
        .write_payload(&display, &mut bytes)
        .expect("custom payload should encode");

    let mut cursor = Cursor::new(bytes.as_slice());
    let id = usize::try_from(VarInt::read(&mut cursor).expect("ID should decode").0)
        .expect("ID is nonnegative");
    let decoded = registry
        .by_id(id)
        .expect("custom ID should resolve")
        .read_payload(&mut cursor)
        .expect("custom payload should decode");
    assert_eq!(decoded, display);
    let cloned = decoded.clone();
    drop(decoded);
    assert_eq!(cloned, display);
    let SlotDisplay::Custom(custom) = cloned else {
        panic!("expected the custom payload")
    };
    assert_eq!(
        custom.downcast_ref::<TestDisplayPayload>(),
        Some(&TestDisplayPayload(400))
    );
    assert!(
        CustomSlotDisplay::new(
            &crate::vanilla_slot_displays::EMPTY,
            TestDisplayPayload(400)
        )
        .is_err()
    );
    assert!(IMPOSTOR.write_payload(&display, &mut Vec::new()).is_err());
    assert!(
        CUSTOM
            .write_payload(&SlotDisplay::Empty, &mut Vec::new())
            .is_err()
    );
    assert!(
        display.write(&mut Vec::new()).is_err(),
        "unpublished types cannot use the global wire registry"
    );
    assert!(CUSTOM.read_payload(&mut Cursor::new(&[])).is_err());
}
