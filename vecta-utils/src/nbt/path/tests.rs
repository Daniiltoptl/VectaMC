use simdnbt::owned::{NbtCompound, NbtList, NbtTag};

use super::*;

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtTag)>) -> NbtTag {
    let mut compound = NbtCompound::new();
    for (key, tag) in entries {
        compound.insert(key, tag);
    }
    NbtTag::Compound(compound)
}

fn list(entries: impl IntoIterator<Item = NbtTag>) -> NbtTag {
    NbtTag::List(NbtList::from(entries.into_iter().collect::<Vec<_>>()))
}

#[test]
fn parses_path_argument_without_consuming_separator() {
    let (path, cursor) = parse_nbt_path_argument("foo.bar run").expect("path parses");

    assert_eq!(path.as_str(), "foo.bar");
    assert_eq!(cursor, 7);
}

#[test]
fn counts_compound_child_matches() {
    let path = parse_nbt_path("foo.bar").expect("path parses");
    let tag = compound([("foo", compound([("bar", NbtTag::Int(3))]))]);

    assert_eq!(path.count_matching(&tag), 1);
    assert_eq!(path.get(&tag), vec![NbtTag::Int(3)]);
}

#[test]
fn counts_list_index_and_wildcard_matches() {
    let path = parse_nbt_path("items[1].id").expect("path parses");
    let wildcard = parse_nbt_path("items[].id").expect("path parses");
    let tag = compound([(
        "items",
        list([
            compound([("id", NbtTag::String("first".into()))]),
            compound([("id", NbtTag::String("second".into()))]),
        ]),
    )]);

    assert_eq!(path.get(&tag), vec![NbtTag::String("second".into())]);
    assert_eq!(wildcard.count_matching(&tag), 2);
}

#[test]
fn negative_indices_select_from_end() {
    let path = parse_nbt_path("items[-1]").expect("path parses");
    let tag = compound([(
        "items",
        list([NbtTag::Int(1), NbtTag::Int(2), NbtTag::Int(3)]),
    )]);

    assert_eq!(path.get(&tag), vec![NbtTag::Int(3)]);
}

#[test]
fn predicate_nodes_use_partial_compound_matching() {
    let path = parse_nbt_path("items[{id:\"minecraft:stone\"}].Count").expect("path parses");
    let tag = compound([(
        "items",
        list([
            compound([
                ("id", NbtTag::String("minecraft:dirt".into())),
                ("Count", NbtTag::Byte(1)),
            ]),
            compound([
                ("id", NbtTag::String("minecraft:stone".into())),
                ("Count", NbtTag::Byte(4)),
                ("Slot", NbtTag::Byte(0)),
            ]),
        ]),
    )]);

    assert_eq!(path.get(&tag), vec![NbtTag::Byte(4)]);
}

#[test]
fn root_predicate_matches_root_compound() {
    let path = parse_nbt_path("{id:\"minecraft:barrel\"}").expect("path parses");
    let tag = compound([
        ("id", NbtTag::String("minecraft:barrel".into())),
        ("x", NbtTag::Int(4)),
    ]);

    assert_eq!(path.count_matching(&tag), 1);
}

#[test]
fn preserves_embedded_snbt_error_kind() {
    let error = parse_nbt_path("items[{id:}]").expect_err("invalid pattern should fail");

    assert_eq!(
        error.kind(),
        &NbtPathErrorKind::InvalidSnbt(SnbtErrorKind::ExpectedValue)
    );
    assert_eq!(
        error.component(),
        TextComponent::from(&translations::SNBT_PARSER_EXPECTED_UNQUOTED_STRING)
    );
}

#[test]
fn quoted_key_errors_use_brigadier_cursors() {
    let unclosed = parse_nbt_path(r#""items"#).expect_err("unclosed key should fail");
    assert_eq!(unclosed.cursor(), r#""items"#.len());
    assert_eq!(unclosed.kind(), &NbtPathErrorKind::UnclosedQuotedString);

    let invalid_escape = parse_nbt_path(r#""a\q""#).expect_err("invalid key escape should fail");
    assert_eq!(invalid_escape.cursor(), r#""a\"#.len());
    assert_eq!(invalid_escape.kind(), &NbtPathErrorKind::InvalidEscape('q'));
}

#[test]
fn index_errors_use_brigadier_integer_components() {
    let expected = parse_nbt_path("items[x]").expect_err("missing integer should fail");
    assert_eq!(expected.cursor(), "items[".len());
    assert_eq!(expected.kind(), &NbtPathErrorKind::ExpectedIndex);
    assert_eq!(
        expected.component(),
        TextComponent::from(&translations::PARSING_INT_EXPECTED)
    );

    for value in ["-", "1.2", "999999999999999999999"] {
        let input = format!("items[{value}]");
        let invalid = parse_nbt_path(&input).expect_err("invalid integer should fail");
        assert_eq!(invalid.cursor(), "items[".len());
        assert_eq!(
            invalid.kind(),
            &NbtPathErrorKind::InvalidIndex(value.to_owned())
        );
        assert_eq!(
            invalid.component(),
            translations::PARSING_INT_INVALID
                .message([value.to_owned()])
                .component()
        );
    }
}

#[test]
fn set_creates_missing_compound_parents() {
    let path = parse_nbt_path("foo.bar").expect("path parses");
    let mut tag = compound([]);

    assert_eq!(
        path.set(&mut tag, NbtTag::Int(7))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(path.get(&tag), vec![NbtTag::Int(7)]);
}

#[test]
fn set_updates_all_list_elements() {
    let path = parse_nbt_path("items[].Count").expect("path parses");
    let mut tag = compound([(
        "items",
        list([
            compound([("Count", NbtTag::Byte(1))]),
            compound([("Count", NbtTag::Byte(2))]),
        ]),
    )]);

    assert_eq!(
        path.set(&mut tag, NbtTag::Byte(4))
            .expect("path set should succeed"),
        2
    );
    assert_eq!(path.get(&tag), vec![NbtTag::Byte(4), NbtTag::Byte(4)]);
}

#[test]
fn set_updates_indexed_array_elements() {
    let path = parse_nbt_path("bytes[1]").expect("path parses");
    let mut tag = compound([("bytes", NbtTag::ByteArray(vec![1, 2, 3]))]);

    assert_eq!(
        path.set(&mut tag, NbtTag::Byte(9))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(path.get(&tag), vec![NbtTag::Byte(9)]);
}

#[test]
fn set_coerces_numeric_array_values_like_vanilla() {
    let mut tag = compound([
        ("bytes", NbtTag::ByteArray(vec![0])),
        ("ints", NbtTag::IntArray(vec![0, 0])),
        ("longs", NbtTag::LongArray(vec![0])),
    ]);

    assert_eq!(
        parse_nbt_path("bytes[0]")
            .expect("path parses")
            .set(&mut tag, NbtTag::Short(258))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(
        parse_nbt_path("ints[]")
            .expect("path parses")
            .set(&mut tag, NbtTag::Float(3.9))
            .expect("path set should succeed"),
        2
    );
    assert_eq!(
        parse_nbt_path("longs[0]")
            .expect("path parses")
            .set(&mut tag, NbtTag::Double(-1.2))
            .expect("path set should succeed"),
        1
    );

    assert_eq!(
        parse_nbt_path("bytes[0]").expect("path parses").get(&tag),
        vec![NbtTag::Byte(2)]
    );
    assert_eq!(
        parse_nbt_path("ints[]").expect("path parses").get(&tag),
        vec![NbtTag::Int(3), NbtTag::Int(3)]
    );
    assert_eq!(
        parse_nbt_path("longs[0]").expect("path parses").get(&tag),
        vec![NbtTag::Long(-2)]
    );
}

#[test]
fn empty_array_wildcard_reports_vanilla_change_count_for_rejected_value() {
    let path = parse_nbt_path("bytes[]").expect("path parses");
    let mut tag = compound([("bytes", NbtTag::ByteArray(vec![]))]);

    assert_eq!(
        path.set(&mut tag, NbtTag::String("not numeric".into()))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(path.get(&tag), Vec::<NbtTag>::new());
}

#[test]
fn set_regular_list_elements_can_change_type_like_vanilla() {
    let path = parse_nbt_path("values[1]").expect("path parses");
    let all_values = parse_nbt_path("values[]").expect("path parses");
    let mut tag = compound([("values", list([NbtTag::Int(1), NbtTag::Int(2)]))]);

    assert_eq!(
        path.set(&mut tag, NbtTag::String("two".into()))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(
        all_values.get(&tag),
        vec![NbtTag::Int(1), NbtTag::String("two".into())]
    );
}

#[test]
fn predicate_parent_creates_match_in_non_compound_list() {
    let path = parse_nbt_path("items[{id:\"minecraft:stone\"}].Count").expect("path parses");
    let mut tag = compound([("items", list([NbtTag::String("existing".into())]))]);

    assert_eq!(
        path.set(&mut tag, NbtTag::Byte(5))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(path.get(&tag), vec![NbtTag::Byte(5)]);
    assert_eq!(
        parse_nbt_path("items[]").expect("path parses").get(&tag),
        vec![
            NbtTag::String("existing".into()),
            compound([
                ("id", NbtTag::String("minecraft:stone".into())),
                ("Count", NbtTag::Byte(5)),
            ]),
        ]
    );
}

#[test]
fn set_predicate_parent_creates_matching_compound() {
    let path = parse_nbt_path("items[{id:\"minecraft:stone\"}].Count").expect("path parses");
    let mut tag = compound([("items", list([]))]);

    assert_eq!(
        path.set(&mut tag, NbtTag::Byte(5))
            .expect("path set should succeed"),
        1
    );
    assert_eq!(path.get(&tag), vec![NbtTag::Byte(5)]);
    assert_eq!(
        parse_nbt_path("items[0].id")
            .expect("id path parses")
            .get(&tag),
        vec![NbtTag::String("minecraft:stone".into())]
    );
}

#[test]
fn set_reports_missing_intermediate_path() {
    let path = parse_nbt_path("items[0].Count").expect("path parses");
    let mut tag = compound([("items", list([]))]);

    assert_eq!(
        path.set(&mut tag, NbtTag::Byte(1)),
        Err(NbtPathMutationError::NothingFound("items[0]".to_owned()))
    );
}

#[test]
fn insert_creates_a_missing_list_and_appends() {
    let path = parse_nbt_path("Items").expect("path parses");
    let mut tag = compound([]);

    assert_eq!(
        path.insert(-1, &mut tag, &[NbtTag::Int(1), NbtTag::Int(2)])
            .expect("insert should succeed"),
        1
    );
    assert_eq!(
        parse_nbt_path("Items[]").expect("path parses").get(&tag),
        vec![NbtTag::Int(1), NbtTag::Int(2)]
    );
}

#[test]
fn insert_places_values_at_vanilla_indices() {
    let path = parse_nbt_path("values").expect("path parses");
    let mut tag = compound([("values", list([NbtTag::Int(1), NbtTag::Int(2)]))]);

    assert_eq!(
        path.insert(0, &mut tag, &[NbtTag::String("first".into())])
            .expect("insert should succeed"),
        1
    );
    assert_eq!(
        path.insert(-2, &mut tag, &[NbtTag::String("middle".into())])
            .expect("insert should succeed"),
        1
    );
    assert_eq!(
        parse_nbt_path("values[]").expect("path parses").get(&tag),
        vec![
            NbtTag::String("first".into()),
            NbtTag::Int(1),
            NbtTag::String("middle".into()),
            NbtTag::Int(2),
        ]
    );
}

#[test]
fn insert_rejects_non_collection_targets_and_out_of_range_indices() {
    let path = parse_nbt_path("value").expect("path parses");
    let mut tag = compound([("value", NbtTag::Int(4))]);
    assert_eq!(
        path.insert(-1, &mut tag, &[NbtTag::Int(1)]),
        Err(NbtPathMutationError::ExpectedList("4".to_owned()))
    );

    let list_path = parse_nbt_path("values").expect("path parses");
    let mut tag = compound([("values", list([NbtTag::Int(1)]))]);
    assert_eq!(
        list_path.insert(5, &mut tag, &[NbtTag::Int(9)]),
        Err(NbtPathMutationError::InvalidIndex(5))
    );
}

#[test]
fn insert_into_an_array_coerces_numeric_values() {
    let path = parse_nbt_path("bytes").expect("path parses");
    let mut tag = compound([("bytes", NbtTag::ByteArray(vec![1, 2]))]);

    assert_eq!(
        path.insert(1, &mut tag, &[NbtTag::Short(258)])
            .expect("insert should succeed"),
        1
    );
    assert_eq!(
        parse_nbt_path("bytes[]").expect("path parses").get(&tag),
        vec![NbtTag::Byte(1), NbtTag::Byte(2), NbtTag::Byte(2)]
    );
}

#[test]
fn insert_skips_incompatible_array_values_without_advancing_the_index() {
    let path = parse_nbt_path("values").expect("path");
    let mut tag = compound([("values", NbtTag::IntArray(vec![10, 20]))]);
    assert_eq!(
        path.insert(
            1,
            &mut tag,
            &[
                NbtTag::String("skip".into()),
                NbtTag::Int(30),
                NbtTag::String("skip again".into()),
                NbtTag::Int(40),
            ]
        ),
        Ok(1)
    );
    assert_eq!(path.get(&tag), vec![NbtTag::IntArray(vec![10, 30, 40, 20])]);
    assert_eq!(
        path.insert(99, &mut tag, &[NbtTag::String("skip".into())]),
        Ok(0)
    );
    assert_eq!(
        path.insert(-99, &mut tag, &[NbtTag::Int(1)]),
        Err(NbtPathMutationError::InvalidIndex(-94))
    );
}

#[test]
fn wildcard_mutations_count_each_changed_target() {
    let path = parse_nbt_path("entries[]").expect("path");
    let mut tag = compound([(
        "entries",
        list([list([NbtTag::Int(1)]), list([NbtTag::Int(2)])]),
    )]);
    assert_eq!(path.insert(-1, &mut tag, &[NbtTag::Int(3)]), Ok(2));
    assert_eq!(
        path.get(&tag),
        vec![
            list([NbtTag::Int(1), NbtTag::Int(3)]),
            list([NbtTag::Int(2), NbtTag::Int(3)]),
        ]
    );

    let mut tag = compound([(
        "entries",
        list([
            compound([("n", NbtTag::Int(1))]),
            compound([("n", NbtTag::Int(1))]),
            compound([("n", NbtTag::Int(2))]),
        ]),
    )]);
    let mut payload = NbtCompound::new();
    payload.insert("n", 2);
    assert_eq!(path.merge(&mut tag, &payload), Ok(2));
    assert!(
        path.get(&tag)
            .iter()
            .all(|value| value == &NbtTag::Compound(payload.clone()))
    );
}

#[test]
fn wildcard_mutations_skip_unresolved_siblings_when_another_parent_matches() {
    let path = parse_nbt_path("entries[].nested.values").expect("path");
    let original = compound([(
        "entries",
        list([NbtTag::Int(5), compound([("nested", compound([]))])]),
    )]);
    let mut inserted = original.clone();
    assert_eq!(path.insert(-1, &mut inserted, &[NbtTag::Int(7)]), Ok(1));
    assert_eq!(path.get(&inserted), vec![list([NbtTag::Int(7)])]);
    let mut assigned = original.clone();
    assert_eq!(path.set(&mut assigned, NbtTag::Int(9)), Ok(1));
    assert_eq!(path.get(&assigned), vec![NbtTag::Int(9)]);
    let mut merged = original;
    let mut payload = NbtCompound::new();
    payload.insert("a", 1);
    assert_eq!(path.merge(&mut merged, &payload), Ok(1));
    assert_eq!(path.get(&merged), vec![NbtTag::Compound(payload)]);
}

#[test]
fn remove_deletes_compound_children_and_collection_entries() {
    let child = parse_nbt_path("keep.drop").expect("path parses");
    let mut tag = compound([(
        "keep",
        compound([("drop", NbtTag::Int(1)), ("stay", NbtTag::Int(2))]),
    )]);
    assert_eq!(child.remove(&mut tag), 1);
    assert_eq!(
        parse_nbt_path("keep").expect("path parses").get(&tag),
        vec![compound([("stay", NbtTag::Int(2))])]
    );

    let element = parse_nbt_path("items[1]").expect("path parses");
    let mut tag = compound([(
        "items",
        list([NbtTag::Int(1), NbtTag::Int(2), NbtTag::Int(3)]),
    )]);
    assert_eq!(element.remove(&mut tag), 1);
    assert_eq!(
        parse_nbt_path("items[]").expect("path parses").get(&tag),
        vec![NbtTag::Int(1), NbtTag::Int(3)]
    );
}

#[test]
fn remove_clears_wildcards_and_reports_unresolved_parents() {
    let wildcard = parse_nbt_path("items[]").expect("path parses");
    let mut tag = compound([(
        "items",
        list([NbtTag::Int(1), NbtTag::Int(2), NbtTag::Int(3)]),
    )]);
    assert_eq!(wildcard.remove(&mut tag), 3);
    assert_eq!(
        parse_nbt_path("items[]").expect("path parses").get(&tag),
        Vec::<NbtTag>::new()
    );

    let missing = parse_nbt_path("absent.child").expect("path parses");
    assert_eq!(missing.remove(&mut tag), 0);
}

#[test]
fn remove_uses_predicate_selection() {
    let path = parse_nbt_path("items[{id:\"minecraft:stone\"}]").expect("path parses");
    let mut tag = compound([(
        "items",
        list([
            compound([("id", NbtTag::String("minecraft:dirt".into()))]),
            compound([("id", NbtTag::String("minecraft:stone".into()))]),
        ]),
    )]);

    assert_eq!(path.remove(&mut tag), 1);
    assert_eq!(
        parse_nbt_path("items[].id").expect("path parses").get(&tag),
        vec![NbtTag::String("minecraft:dirt".into())]
    );
}

#[test]
fn merge_applies_vanilla_compound_merge_to_each_target() {
    let path = parse_nbt_path("entries[{id:\"a\"}]").expect("path parses");
    let mut tag = compound([(
        "entries",
        list([compound([
            ("id", NbtTag::String("a".into())),
            ("count", NbtTag::Int(1)),
        ])]),
    )]);
    let mut source = NbtCompound::new();
    source.insert("count", NbtTag::Int(2));
    source.insert("extra", NbtTag::String("x".into()));

    assert_eq!(
        path.merge(&mut tag, &source).expect("merge should succeed"),
        1
    );
    assert_eq!(
        parse_nbt_path("entries[0].count")
            .expect("path parses")
            .get(&tag),
        vec![NbtTag::Int(2)]
    );
    assert_eq!(
        parse_nbt_path("entries[0].extra")
            .expect("path parses")
            .get(&tag),
        vec![NbtTag::String("x".into())]
    );
    assert_eq!(
        path.merge(&mut tag, &source).expect("merge should succeed"),
        0,
        "an unchanged merge reports no modification"
    );
}

#[test]
fn merge_creates_a_missing_compound_target() {
    let path = parse_nbt_path("absent").expect("path parses");
    let mut tag = compound([]);
    let mut source = NbtCompound::new();
    source.insert("value", NbtTag::Int(7));

    assert_eq!(
        path.merge(&mut tag, &source).expect("merge should succeed"),
        1
    );
    assert_eq!(
        parse_nbt_path("absent.value")
            .expect("path parses")
            .get(&tag),
        vec![NbtTag::Int(7)]
    );
}

#[test]
fn merge_checks_source_depth_independently_of_target_path_depth() {
    let mut source = NbtCompound::new();
    source.insert("n", 1);
    for _ in 0..510 {
        let mut parent = NbtCompound::new();
        parent.insert("child", source);
        source = parent;
    }
    assert!(!is_too_deep(&NbtTag::Compound(source.clone()), 0));
    let path = parse_nbt_path("nested.target").expect("path");
    let mut root = compound([]);
    assert_eq!(path.merge(&mut root, &source), Ok(1));
}
