//! Vanilla `/data` command.

mod accessor;

use simdnbt::{
    Mutf8Str, Mutf8String,
    owned::{NbtCompound, NbtTag},
};
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
use crate::block_entity::BlockEntity;
use crate::command::builtins::execute::loaded_block_position;
use crate::entity::{entity_loot_ref, loot_equipment};
use vecta_registry::loot_table::{BlockEntityRef, ContextKeySet, LootContext, loot_context_params};
use vecta_registry::vanilla_context_key_sets as loot_context_param_sets;

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

    parent
        .then(from)
        .then(string)
        .then(compute_branches(provider, manipulator))
        .then(
            literal("value").then(argument("value", VectaArgumentType::nbt_tag()).executes(
                move |context| manipulate(context, provider, manipulator, Source::Value),
            )),
        )
}

/// The `compute` source: `default`, `block <pos>` and `entity <target>` context
/// sources followed by `float`/`integer` and a `ResourceOrIdArgument` provider,
/// mirroring `LootContextSources.addContextSources`.
fn compute_branches(provider: Provider, manipulator: Manipulator) -> Builder {
    let compute = |source: ComputeSource, parent: Builder| {
        parent
            .then(literal("float").then(
                argument("provider", VectaArgumentType::context_float_provider()).executes(
                    move |context| {
                        manipulate_compute(
                            context,
                            provider,
                            manipulator,
                            source,
                            ComputeFamily::Float,
                        )
                    },
                ),
            ))
            .then(literal("integer").then(
                argument("provider", VectaArgumentType::context_int_provider()).executes(
                    move |context| {
                        manipulate_compute(
                            context,
                            provider,
                            manipulator,
                            source,
                            ComputeFamily::Integer,
                        )
                    },
                ),
            ))
    };

    literal("compute")
        .then(compute(ComputeSource::Default, literal("default")))
        .then(literal("block").then(compute(
            ComputeSource::Block,
            argument("computePos", VectaArgumentType::block_pos()),
        )))
        .then(literal("entity").then(compute(
            ComputeSource::Entity,
            argument("computeTarget", VectaArgumentType::entity()),
        )))
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

/// The `compute` context source: which `LootContextSources` branch was used.
#[derive(Clone, Copy)]
enum ComputeSource {
    Default,
    Block,
    Entity,
}

/// The `compute` result family.
#[derive(Clone, Copy)]
enum ComputeFamily {
    Float,
    Integer,
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
    apply_source(context, target, manipulator, source_tags)
}

/// Mutates the target path with the resolved source tags and reports the change.
fn apply_source(
    context: &VectaCommandContext<CommandSource>,
    target: Provider,
    manipulator: Manipulator,
    source_tags: Vec<NbtTag>,
) -> Result<i32, CommandSyntaxError> {
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

fn manipulate_compute(
    context: &VectaCommandContext<CommandSource>,
    target: Provider,
    manipulator: Manipulator,
    source: ComputeSource,
    family: ComputeFamily,
) -> Result<i32, CommandSyntaxError> {
    let source_stack = context.source();
    let world = source_stack.world();
    let origin = source_stack.position();
    let this_entity = source_stack.entity();

    // Java evaluates inside the level's own mutable random, so the whole
    // evaluation stays within one borrow of that stream.
    world.random().with_level(|rng| {
        // Every borrowed parameter is bound at the top of this closure so it
        // outlives the `LootContext`, which keeps references into its values.
        let target_entity = match source {
            ComputeSource::Entity => Some(context.entity("computeTarget")?),
            _ => None,
        };
        let target_equipment = target_entity
            .as_ref()
            .and_then(|entity| loot_equipment(entity.as_ref()));
        let this_equipment = this_entity
            .as_ref()
            .and_then(|entity| loot_equipment(entity.as_ref()));

        let position = match source {
            ComputeSource::Block => Some(loaded_block_position(context, "computePos")?),
            _ => None,
        };
        let block_entity = position
            .as_ref()
            .and_then(|pos| world.get_block_entity(*pos));
        let block_entity_ref = block_entity.as_deref().map(block_entity_loot_ref);

        let mut loot = LootContext::new(rng)
            .with_parameter(loot_context_params::ORIGIN, (origin.x, origin.y, origin.z))
            .with_data(world.as_ref());
        if let Some(entity) = this_entity {
            loot = loot.with_parameter(
                loot_context_params::THIS_ENTITY,
                entity_loot_ref(entity.as_ref(), this_equipment.as_ref()),
            );
        }

        match source {
            ComputeSource::Default => {
                validate_compute_context(&loot, &loot_context_param_sets::COMMAND_COMPUTE_DEFAULT)?;
            }
            ComputeSource::Block => {
                if let Some(position) = position {
                    loot = loot.with_parameter(
                        loot_context_params::BLOCK_STATE,
                        world.get_block_state(position),
                    );
                }
                if let Some(block_entity_ref) = block_entity_ref {
                    loot = loot.with_parameter(loot_context_params::BLOCK_ENTITY, block_entity_ref);
                }
                validate_compute_context(
                    &loot,
                    &loot_context_param_sets::COMMAND_COMPUTE_POSITION,
                )?;
            }
            ComputeSource::Entity => {
                if let Some(entity) = target_entity.as_ref() {
                    loot = loot.with_parameter(
                        loot_context_params::TARGET_ENTITY,
                        entity_loot_ref(entity.as_ref(), target_equipment.as_ref()),
                    );
                }
                validate_compute_context(&loot, &loot_context_param_sets::COMMAND_COMPUTE_ENTITY)?;
            }
        }

        let tag = match family {
            ComputeFamily::Float => {
                let value = context
                    .context_float_provider("provider")?
                    .get_float(&mut loot);
                NbtTag::Float(value)
            }
            ComputeFamily::Integer => {
                let value = context.context_int_provider("provider")?.get_int(&mut loot);
                NbtTag::Int(value)
            }
        };
        apply_source(context, target, manipulator, vec![tag])
    })
}

fn validate_compute_context(
    loot: &LootContext<'_>,
    set: &ContextKeySet,
) -> Result<(), CommandSyntaxError> {
    loot.params
        .validate(set)
        .map_err(|error| CommandSyntaxError::dynamic(TextComponent::from(error.to_string())))
}

/// A read-only view of a block entity for loot parameters.
fn block_entity_loot_ref(block_entity: &dyn BlockEntity) -> BlockEntityRef<'_> {
    BlockEntityRef {
        block_entity_type: Some(&block_entity.get_type().key),
        custom_name: None,
        inventory: None,
    }
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
        NbtTag::String(value) => i32::try_from(
            value
                .as_bytes()
                .iter()
                .filter(|byte| **byte & 0xc0 != 0x80)
                .count(),
        )
        .unwrap_or(i32::MAX),
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
    Ok(NbtTag::String(processed))
}

fn tag_as_text(tag: &NbtTag) -> Result<Mutf8String, CommandSyntaxError> {
    let text = match tag {
        NbtTag::String(value) => return Ok(value.clone()),
        NbtTag::Byte(value) => format!("{value}b"),
        NbtTag::Short(value) => format!("{value}s"),
        NbtTag::Int(value) => value.to_string(),
        NbtTag::Long(value) => format!("{value}L"),
        NbtTag::Float(_) | NbtTag::Double(_) => accessor::render_tag(tag),
        other => {
            return Err(CommandSyntaxError::dynamic(
                translations::COMMANDS_DATA_MODIFY_EXPECTED_VALUE
                    .message([TextComponent::plain(accessor::render_tag(other))])
                    .component(),
            ));
        }
    };
    Ok(text.into())
}

/// Vanilla `DataCommands.substring` over UTF-16 code units, as Java indexes strings.
fn substring(
    input: &Mutf8Str,
    start: i32,
    end: Option<i32>,
) -> Result<Mutf8String, CommandSyntaxError> {
    // MUTF-8 encodes each UTF-16 unit separately, including unpaired surrogates.
    let mut boundaries: Vec<usize> = input
        .as_bytes()
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte & 0xc0 != 0x80).then_some(index))
        .collect();
    boundaries.push(input.len());
    let length = i32::try_from(boundaries.len() - 1).unwrap_or(i32::MAX);
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
    let to = usize::try_from(absolute_end).unwrap_or(boundaries.len() - 1);
    Ok(Mutf8Str::from_slice(&input.as_bytes()[boundaries[from]..boundaries[to]]).to_owned())
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
        for (value, expected) in [
            (NbtTag::Float(0.1), "0.1f"),
            (NbtTag::Float(-0.0), "-0.0f"),
            (NbtTag::Float(1.0e7), "1.0E7f"),
            (NbtTag::Double(1.0e-4), "1.0E-4d"),
        ] {
            assert_eq!(
                stringify(&value, None, None).expect("numeric text"),
                NbtTag::String(expected.into())
            );
        }
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
        let text = NbtTag::String("a😀z".into());
        let high = stringify(&text, Some(1), Some(2)).expect("high surrogate");
        let low = stringify(&text, Some(2), Some(3)).expect("low surrogate");
        assert_eq!(
            high,
            NbtTag::String(Mutf8Str::from_slice(&[0xed, 0xa0, 0xbd]).to_owned())
        );
        assert_eq!(
            low,
            NbtTag::String(Mutf8Str::from_slice(&[0xed, 0xb8, 0x80]).to_owned())
        );
        assert_eq!(tag_value(&high), 1);
        assert_eq!(
            stringify(&high, None, None).expect("preserve surrogate"),
            high
        );
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
