//! Vanilla `/data` command.

mod accessor;

use simdnbt::owned::{NbtCompound, NbtTag};
use text_components::TextComponent;
use vecta_utils::{
    Identifier,
    nbt::{NbtPath, NbtPathMutationError, is_too_deep, merge_nbt_compounds, nbt_list_values},
    translations,
};

use self::accessor::Provider;
use super::super::{
    brigadier::{ArgumentType, CommandNodeBuilder, CommandSyntaxError},
    execution::{
        CommandSource, VectaArgumentType, VectaCommandContext, VectaCommandRuntime, argument,
        literal,
    },
    registration::CommandRegistration,
};

type Builder = CommandNodeBuilder<CommandSource, VectaCommandRuntime>;

pub(super) fn registration() -> CommandRegistration<CommandSource> {
    CommandRegistration::new(Identifier::vanilla_static("data"), |_| command())
}

fn command() -> Builder {
    literal("data")
        .then(target_branches(literal("merge"), |provider, node| {
            node.then(
                argument("nbt", VectaArgumentType::compound_tag())
                    .executes(move |context| merge_data(context, provider)),
            )
        }))
        .then(target_branches(literal("get"), |provider, node| {
            node.executes(move |context| get_data(context, provider))
                .then(
                    argument("path", VectaArgumentType::nbt_path())
                        .executes(move |context| get_path(context, provider))
                        .then(
                            argument("scale", ArgumentType::double(f64::MIN, f64::MAX))
                                .executes(move |context| get_numeric(context, provider)),
                        ),
                )
        }))
        .then(target_branches(literal("remove"), |provider, node| {
            node.then(
                argument("path", VectaArgumentType::nbt_path())
                    .executes(move |context| remove_data(context, provider)),
            )
        }))
        .then(target_branches(literal("modify"), |provider, node| {
            node.then(manipulator_branches(
                argument("targetPath", VectaArgumentType::nbt_path()),
                provider,
            ))
        }))
}

/// Attaches one provider literal and `target` argument per provider, mirroring
/// vanilla's `ArgProvider.wrap` loop over `TARGET_PROVIDERS`.
fn target_branches(parent: Builder, decorate: impl Fn(Provider, Builder) -> Builder) -> Builder {
    Provider::ALL.into_iter().fold(parent, |parent, provider| {
        let node = argument("target", provider.argument_type());
        parent.then(literal(provider.name()).then(decorate(provider, node)))
    })
}

/// The `insert`/`prepend`/`append`/`set`/`merge` manipulators.
fn manipulator_branches(parent: Builder, provider: Provider) -> Builder {
    let insert = literal("insert").then(source_branches(
        argument("index", ArgumentType::integer(i32::MIN, i32::MAX)),
        provider,
        Manipulator::InsertIndex,
    ));
    parent
        .then(insert)
        .then(source_branches(
            literal("prepend"),
            provider,
            Manipulator::Insert(0),
        ))
        .then(source_branches(
            literal("append"),
            provider,
            Manipulator::Insert(-1),
        ))
        .then(source_branches(literal("set"), provider, Manipulator::Set))
        .then(source_branches(
            literal("merge"),
            provider,
            Manipulator::Merge,
        ))
}

/// Attaches `from`/`string`/`value` source branches to a manipulator node.
///
/// Vanilla's `ArgProvider.wrap` puts the provider literal under `from`/`string`,
/// so the order is `<manipulator> from <provider> <source>`.
fn source_branches(parent: Builder, provider: Provider, manipulator: Manipulator) -> Builder {
    let from = Provider::ALL
        .into_iter()
        .fold(literal("from"), |from, source| {
            from.then(
                literal(source.name()).then(
                    argument("source", source.argument_type())
                        .executes(move |context| {
                            manipulate(context, provider, manipulator, Source::From(source))
                        })
                        .then(
                            argument("sourcePath", VectaArgumentType::nbt_path()).executes(
                                move |context| {
                                    manipulate(context, provider, manipulator, Source::From(source))
                                },
                            ),
                        ),
                ),
            )
        });

    let string = Provider::ALL
        .into_iter()
        .fold(literal("string"), |string, source| {
            string.then(
                literal(source.name()).then(
                    argument("source", source.argument_type())
                        .executes(move |context| {
                            manipulate(context, provider, manipulator, Source::String(source))
                        })
                        .then(
                            argument("sourcePath", VectaArgumentType::nbt_path())
                                .executes(move |context| {
                                    manipulate(
                                        context,
                                        provider,
                                        manipulator,
                                        Source::String(source),
                                    )
                                })
                                .then(
                                    argument("start", ArgumentType::integer(i32::MIN, i32::MAX))
                                        .executes(move |context| {
                                            manipulate(
                                                context,
                                                provider,
                                                manipulator,
                                                Source::String(source),
                                            )
                                        })
                                        .then(
                                            argument(
                                                "end",
                                                ArgumentType::integer(i32::MIN, i32::MAX),
                                            )
                                            .executes(
                                                move |context| {
                                                    manipulate(
                                                        context,
                                                        provider,
                                                        manipulator,
                                                        Source::String(source),
                                                    )
                                                },
                                            ),
                                        ),
                                ),
                        ),
                ),
            )
        });

    parent.then(from).then(string).then(
        literal("value").then(
            argument("value", VectaArgumentType::nbt_tag())
                .executes(move |context| manipulate(context, provider, manipulator, Source::Value)),
        ),
    )
}

#[derive(Clone, Copy)]
enum Manipulator {
    /// `insert <index>` reads the index from the command context.
    InsertIndex,
    /// `prepend` (0) and `append` (-1) use a fixed vanilla index.
    Insert(i32),
    Set,
    Merge,
}

#[derive(Clone, Copy)]
enum Source {
    From(Provider),
    String(Provider),
    Value,
}

fn merge_data(
    context: &VectaCommandContext<CommandSource>,
    provider: Provider,
) -> Result<i32, CommandSyntaxError> {
    let accessor = provider.resolve(context, "target")?;
    let payload = context.compound_tag("nbt")?;
    if compound_is_too_deep(payload) {
        return Err(too_deep());
    }

    let old = accessor.get_data(context.source())?;
    let mut merged = old.clone();
    merge_nbt_compounds(&mut merged, payload);
    if merged == old {
        return Err(merge_failed());
    }

    accessor.set_data(context.source(), merged)?;
    context
        .source()
        .send_success(&accessor.modified_success(), true);
    Ok(1)
}

fn get_data(
    context: &VectaCommandContext<CommandSource>,
    provider: Provider,
) -> Result<i32, CommandSyntaxError> {
    let accessor = provider.resolve(context, "target")?;
    let data = accessor.get_data(context.source())?;
    context
        .source()
        .send_success(&accessor.print_success(&NbtTag::Compound(data)), false);
    Ok(1)
}

fn get_path(
    context: &VectaCommandContext<CommandSource>,
    provider: Provider,
) -> Result<i32, CommandSyntaxError> {
    let accessor = provider.resolve(context, "target")?;
    let path = context.nbt_path("path")?.clone();
    let data = accessor.get_data(context.source())?;
    let tag = single_tag(&path, data)?;

    let value = tag_value(&tag);
    context
        .source()
        .send_success(&accessor.print_success(&tag), false);
    Ok(value)
}

fn get_numeric(
    context: &VectaCommandContext<CommandSource>,
    provider: Provider,
) -> Result<i32, CommandSyntaxError> {
    let accessor = provider.resolve(context, "target")?;
    let path = context.nbt_path("path")?.clone();
    let scale = context.double("scale")?;
    let data = accessor.get_data(context.source())?;
    let tag = single_tag(&path, data)?;

    let Some(number) = tag_number(&tag) else {
        return Err(CommandSyntaxError::dynamic(
            translations::COMMANDS_DATA_GET_INVALID
                .message([TextComponent::plain(path.as_str().to_owned())])
                .component(),
        ));
    };
    let value = (number * scale).floor() as i32;
    context.source().send_success(
        &accessor.print_success_scaled(path.as_str(), scale, value),
        false,
    );
    Ok(value)
}

fn remove_data(
    context: &VectaCommandContext<CommandSource>,
    provider: Provider,
) -> Result<i32, CommandSyntaxError> {
    let accessor = provider.resolve(context, "target")?;
    let path = context.nbt_path("path")?.clone();
    let mut root = NbtTag::Compound(accessor.get_data(context.source())?);

    let count = path.remove(&mut root);
    if count == 0 {
        return Err(merge_failed());
    }

    accessor.set_data(context.source(), into_compound(root)?)?;
    context
        .source()
        .send_success(&accessor.modified_success(), true);
    Ok(i32::try_from(count).unwrap_or(i32::MAX))
}

fn manipulate(
    context: &VectaCommandContext<CommandSource>,
    target: Provider,
    manipulator: Manipulator,
    source: Source,
) -> Result<i32, CommandSyntaxError> {
    let source_tags = resolve_source(context, source)?;
    let accessor = target.resolve(context, "target")?;
    let path = context.nbt_path("targetPath")?.clone();
    let mut root = NbtTag::Compound(accessor.get_data(context.source())?);

    let result = match manipulator {
        Manipulator::InsertIndex => {
            let index = context.integer("index")?;
            path.insert(index, &mut root, &source_tags)
        }
        Manipulator::Insert(index) => path.insert(index, &mut root, &source_tags),
        Manipulator::Set => {
            let value = source_tags.last().cloned().ok_or_else(|| {
                CommandSyntaxError::dynamic(
                    translations::ARGUMENTS_NBTPATH_NOTHING_FOUND
                        .message([TextComponent::plain(path.as_str().to_owned())])
                        .component(),
                )
            })?;
            path.set(&mut root, value)
        }
        Manipulator::Merge => {
            let mut combined = NbtCompound::new();
            for tag in &source_tags {
                if is_too_deep(tag, 0) {
                    return Err(too_deep());
                }
                let NbtTag::Compound(compound) = tag else {
                    return Err(CommandSyntaxError::dynamic(
                        translations::COMMANDS_DATA_MODIFY_EXPECTED_OBJECT
                            .message([TextComponent::plain(accessor::render_tag(tag))])
                            .component(),
                    ));
                };
                merge_nbt_compounds(&mut combined, compound);
            }
            path.merge(&mut root, &combined)
        }
    }
    .map_err(path_mutation_error)?;

    if result == 0 {
        return Err(merge_failed());
    }

    accessor.set_data(context.source(), into_compound(root)?)?;
    context
        .source()
        .send_success(&accessor.modified_success(), true);
    Ok(i32::try_from(result).unwrap_or(i32::MAX))
}

fn resolve_source(
    context: &VectaCommandContext<CommandSource>,
    source: Source,
) -> Result<Vec<NbtTag>, CommandSyntaxError> {
    match source {
        Source::Value => Ok(vec![context.nbt_tag("value")?.clone()]),
        Source::From(provider) => {
            let accessor = provider.resolve(context, "source")?;
            let data = NbtTag::Compound(accessor.get_data(context.source())?);
            let Some(path) = optional_path(context, "sourcePath") else {
                return Ok(vec![data]);
            };
            let tags = path.get(&data);
            if tags.is_empty() {
                return Err(nothing_found(path));
            }
            Ok(tags)
        }
        Source::String(provider) => {
            let accessor = provider.resolve(context, "source")?;
            let data = NbtTag::Compound(accessor.get_data(context.source())?);
            let tags = match optional_path(context, "sourcePath") {
                Some(path) => {
                    let tags = path.get(&data);
                    if tags.is_empty() {
                        return Err(nothing_found(path));
                    }
                    tags
                }
                None => vec![data],
            };

            let start = context.integer("start").ok();
            let end = context.integer("end").ok();
            tags.iter().map(|tag| stringify(tag, start, end)).collect()
        }
    }
}

fn optional_path<'a>(
    context: &'a VectaCommandContext<CommandSource>,
    name: &str,
) -> Option<&'a NbtPath> {
    // The command graph only offers these nodes when the argument is present.
    context.nbt_path(name).ok()
}

fn single_tag(path: &NbtPath, data: NbtCompound) -> Result<NbtTag, CommandSyntaxError> {
    let mut tags = path.get(&NbtTag::Compound(data)).into_iter();
    match (tags.next(), tags.next()) {
        (Some(tag), None) => Ok(tag),
        (None, _) => Err(nothing_found(path)),
        (Some(_), Some(_)) => Err(CommandSyntaxError::dynamic(TextComponent::from(
            &translations::COMMANDS_DATA_GET_MULTIPLE,
        ))),
    }
}

/// Vanilla `DataCommands.getData` result value for a selected tag.
fn tag_value(tag: &NbtTag) -> i32 {
    match tag {
        NbtTag::Byte(value) => i32::from(*value),
        NbtTag::Short(value) => i32::from(*value),
        NbtTag::Int(value) => *value,
        NbtTag::Long(value) => (*value as f64).floor() as i32,
        NbtTag::Float(value) => f64::from(*value).floor() as i32,
        NbtTag::Double(value) => value.floor() as i32,
        NbtTag::String(value) => {
            i32::try_from(value.to_string_lossy().encode_utf16().count()).unwrap_or(i32::MAX)
        }
        NbtTag::List(list) => i32::try_from(nbt_list_values(list).len()).unwrap_or(i32::MAX),
        NbtTag::ByteArray(values) => i32::try_from(values.len()).unwrap_or(i32::MAX),
        NbtTag::IntArray(values) => i32::try_from(values.len()).unwrap_or(i32::MAX),
        NbtTag::LongArray(values) => i32::try_from(values.len()).unwrap_or(i32::MAX),
        NbtTag::Compound(compound) => i32::try_from(compound.len()).unwrap_or(i32::MAX),
    }
}

fn tag_number(tag: &NbtTag) -> Option<f64> {
    Some(match tag {
        NbtTag::Byte(value) => f64::from(*value),
        NbtTag::Short(value) => f64::from(*value),
        NbtTag::Int(value) => f64::from(*value),
        NbtTag::Long(value) => *value as f64,
        NbtTag::Float(value) => f64::from(*value),
        NbtTag::Double(value) => *value,
        _ => return None,
    })
}

/// Vanilla `DataCommands.getAsText` plus the optional `string` substring.
fn stringify(
    tag: &NbtTag,
    start: Option<i32>,
    end: Option<i32>,
) -> Result<NbtTag, CommandSyntaxError> {
    let text = tag_as_text(tag)?;
    let processed = match (start, end) {
        (None, _) => text,
        (Some(start), None) => substring(&text, start, None)?,
        (Some(start), Some(end)) => substring(&text, start, Some(end))?,
    };
    Ok(NbtTag::String(processed.into()))
}

fn tag_as_text(tag: &NbtTag) -> Result<String, CommandSyntaxError> {
    let text = match tag {
        NbtTag::String(value) => value.to_string(),
        NbtTag::Byte(value) => format!("{value}b"),
        NbtTag::Short(value) => format!("{value}s"),
        NbtTag::Int(value) => value.to_string(),
        NbtTag::Long(value) => format!("{value}L"),
        NbtTag::Float(value) => format!("{}f", java_float_text(f64::from(*value))),
        NbtTag::Double(value) => format!("{}d", java_float_text(*value)),
        other => {
            return Err(CommandSyntaxError::dynamic(
                translations::COMMANDS_DATA_MODIFY_EXPECTED_VALUE
                    .message([TextComponent::plain(accessor::render_tag(other))])
                    .component(),
            ));
        }
    };
    Ok(text)
}

/// Formats a float the way Java's `Double.toString` does, keeping the trailing
/// `.0` that Rust omits for integral values.
fn java_float_text(value: f64) -> String {
    let text = value.to_string();
    if text.contains(['.', 'e', 'E']) || text.contains("inf") || text.contains("NaN") {
        text
    } else {
        format!("{text}.0")
    }
}

/// Vanilla `DataCommands.substring` over UTF-16 code units, as Java indexes strings.
fn substring(input: &str, start: i32, end: Option<i32>) -> Result<String, CommandSyntaxError> {
    let units: Vec<u16> = input.encode_utf16().collect();
    let length = i32::try_from(units.len()).unwrap_or(i32::MAX);
    let absolute_start = if start >= 0 { start } else { length + start };
    let absolute_end = match end {
        Some(end) if end >= 0 => end,
        Some(end) => length + end,
        None => length,
    };

    if absolute_start < 0 || absolute_end > length || absolute_start > absolute_end {
        return Err(CommandSyntaxError::dynamic(
            translations::COMMANDS_DATA_MODIFY_INVALID_SUBSTRING
                .message([
                    TextComponent::plain(absolute_start.to_string()),
                    TextComponent::plain(absolute_end.to_string()),
                ])
                .component(),
        ));
    }

    let from = usize::try_from(absolute_start).unwrap_or(0);
    let to = usize::try_from(absolute_end).unwrap_or(units.len());
    Ok(String::from_utf16_lossy(&units[from..to]))
}

fn into_compound(root: NbtTag) -> Result<NbtCompound, CommandSyntaxError> {
    match root {
        NbtTag::Compound(compound) => Ok(compound),
        _ => Err(CommandSyntaxError::dynamic(
            "NBT target root is not a compound",
        )),
    }
}

fn compound_is_too_deep(compound: &NbtCompound) -> bool {
    compound.iter().any(|(_, tag)| is_too_deep(tag, 1))
}

fn nothing_found(path: &NbtPath) -> CommandSyntaxError {
    CommandSyntaxError::dynamic(
        translations::ARGUMENTS_NBTPATH_NOTHING_FOUND
            .message([TextComponent::plain(path.as_str().to_owned())])
            .component(),
    )
}

fn too_deep() -> CommandSyntaxError {
    CommandSyntaxError::dynamic(TextComponent::from(
        &translations::ARGUMENTS_NBTPATH_TOO_DEEP,
    ))
}

fn merge_failed() -> CommandSyntaxError {
    CommandSyntaxError::dynamic(TextComponent::from(
        &translations::COMMANDS_DATA_MERGE_FAILED,
    ))
}

fn path_mutation_error(error: NbtPathMutationError) -> CommandSyntaxError {
    match error {
        NbtPathMutationError::NothingFound(path) => CommandSyntaxError::dynamic(
            translations::ARGUMENTS_NBTPATH_NOTHING_FOUND
                .message([TextComponent::plain(path)])
                .component(),
        ),
        NbtPathMutationError::TooDeep => too_deep(),
        NbtPathMutationError::ExpectedList(tag) => CommandSyntaxError::dynamic(
            translations::COMMANDS_DATA_MODIFY_EXPECTED_LIST
                .message([TextComponent::plain(tag)])
                .component(),
        ),
        NbtPathMutationError::ExpectedObject(tag) => CommandSyntaxError::dynamic(
            translations::COMMANDS_DATA_MODIFY_EXPECTED_OBJECT
                .message([TextComponent::plain(tag)])
                .component(),
        ),
        NbtPathMutationError::InvalidIndex(index) => CommandSyntaxError::dynamic(
            translations::COMMANDS_DATA_MODIFY_INVALID_INDEX
                .message([TextComponent::plain(index.to_string())])
                .component(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use simdnbt::owned::NbtList;

    use super::super::create_dispatcher;
    use super::*;
    use crate::command::brigadier::{CommandDispatcher, NodeId};

    fn child_names(
        dispatcher: &CommandDispatcher<CommandSource, VectaCommandRuntime>,
        node: NodeId,
    ) -> Vec<String> {
        dispatcher.children(node).map_or_else(Vec::new, |children| {
            children
                .iter()
                .filter_map(|child| dispatcher.node(*child))
                .map(|node| node.name().to_owned())
                .collect()
        })
    }

    #[test]
    fn tag_values_match_vanilla_result_types() {
        assert_eq!(tag_value(&NbtTag::Int(7)), 7);
        assert_eq!(tag_value(&NbtTag::Byte(-3)), -3);
        assert_eq!(tag_value(&NbtTag::Double(2.9)), 2);
        assert_eq!(
            tag_value(&NbtTag::Long(4_294_967_296)),
            i32::MAX,
            "Java's `Mth.floor` clamps an out-of-range double to the int range"
        );
        assert_eq!(tag_value(&NbtTag::String("abcd".into())), 4);
        assert_eq!(tag_value(&NbtTag::List(NbtList::Int(vec![1, 2, 3]))), 3);
        let mut compound = NbtCompound::new();
        compound.insert("a", 1);
        assert_eq!(tag_value(&NbtTag::Compound(compound)), 1);
    }

    #[test]
    fn string_sources_use_vanilla_numeric_suffixes() {
        assert_eq!(
            stringify(&NbtTag::Byte(1), None, None).expect("byte text"),
            NbtTag::String("1b".into())
        );
        assert_eq!(
            stringify(&NbtTag::Long(2), None, None).expect("long text"),
            NbtTag::String("2L".into())
        );
        assert_eq!(
            stringify(&NbtTag::Float(1.5), None, None).expect("float text"),
            NbtTag::String("1.5f".into())
        );
        assert_eq!(
            stringify(&NbtTag::Double(1.0), None, None).expect("double text"),
            NbtTag::String("1.0d".into()),
            "Java keeps the trailing `.0` that Rust omits"
        );
        assert!(
            stringify(&NbtTag::List(NbtList::Int(vec![1])), None, None).is_err(),
            "a non-primitive tag is rejected like `commands.data.modify.expected_value`"
        );
    }

    #[test]
    fn string_substrings_use_utf16_indices_and_reject_out_of_range() {
        assert_eq!(
            stringify(&NbtTag::String("abcdef".into()), Some(1), Some(4)).expect("substring"),
            NbtTag::String("bcd".into())
        );
        assert_eq!(
            stringify(&NbtTag::String("abcdef".into()), Some(-2), None).expect("suffix"),
            NbtTag::String("ef".into())
        );
        assert!(
            stringify(&NbtTag::String("abc".into()), Some(2), Some(1)).is_err(),
            "an inverted range is `commands.data.modify.invalid_substring`"
        );
    }

    #[test]
    fn data_graph_matches_vanilla_targets_and_manipulators() {
        vecta_registry::init_vanilla_registry();
        let dispatcher = create_dispatcher().expect("built-in commands should register");
        let roots = child_names(&dispatcher, dispatcher.root());
        assert!(
            roots.iter().any(|name| name == "data"),
            "`data` should be registered, got {roots:?}"
        );
        let mut node = dispatcher.root();
        for name in [
            "data",
            "modify",
            "block",
            "target",
            "targetPath",
            "append",
            "from",
            "block",
            "source",
        ] {
            let Some(child) = dispatcher.children(node).and_then(|children| {
                children.iter().copied().find(|child| {
                    dispatcher
                        .node(*child)
                        .is_some_and(|node| node.name() == name)
                })
            }) else {
                let children = child_names(&dispatcher, node);
                panic!("`{name}` should be reachable, children are {children:?}");
            };
            node = child;
        }
    }
}
