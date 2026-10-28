//! Vanilla damage entity command.

use std::sync::Arc;

use super::super::{
    brigadier::{ArgumentType, CommandNodeBuilder, CommandSyntaxError},
    execution::{
        CommandSource, VectaArgumentType, VectaCommandContext, VectaCommandRuntime, argument,
        literal,
    },
    registration::CommandRegistration,
};
use crate::entity::damage::DamageSource;
use text_components::TextComponent;
use vecta_registry::vanilla_damage_types;
use vecta_utils::Identifier;
use vecta_utils::translations::{COMMANDS_DAMAGE_INVULNERABLE, COMMANDS_DAMAGE_SUCCESS};

pub(super) fn registration() -> CommandRegistration<CommandSource> {
    CommandRegistration::new(Identifier::vanilla_static("damage"), |_| command())
}

fn command() -> CommandNodeBuilder<CommandSource, VectaCommandRuntime> {
    literal("damage").then(
        argument("target", VectaArgumentType::entity()).then(
            argument("amount", ArgumentType::float(0.0, f32::MAX))
                .executes(damage)
                .then(
                    argument("damageType", VectaArgumentType::damage_type())
                        .executes(damage)
                        .then(literal("at").then(
                            argument("location", VectaArgumentType::vec3(true)).executes(damage),
                        ))
                        .then(
                            literal("by").then(
                                argument("entity", VectaArgumentType::entity())
                                    .executes(damage)
                                    .then(
                                        literal("from").then(
                                            argument("cause", VectaArgumentType::entity())
                                                .executes(damage),
                                        ),
                                    ),
                            ),
                        ),
                ),
        ),
    )
}

fn damage(context: &VectaCommandContext<CommandSource>) -> Result<i32, CommandSyntaxError> {
    let target = context.entity("target")?;
    let amount = context.float("amount")?;

    // The base damage type for this command is generic
    let damage_type = context
        .damage_type("damageType")
        .unwrap_or(&vanilla_damage_types::GENERIC);

    // Create the DamageSource to apply modifiers after
    let mut damage_source = DamageSource::environment(damage_type);

    // If we can get "location" from the context, it's from "at"
    if let Ok(coordinates) = context.coordinates("location") {
        damage_source = damage_source.with_source_position(coordinates.position(context.source()));
    }

    // Else, it's from the "by", or maybe it's nothing
    if let Ok(entity) = context.entity("entity") {
        damage_source = damage_source
            .with_direct_entity(Arc::clone(&entity))
            .with_causing_entity(entity);

        // Maybe even the causing entity is known
        if let Ok(cause) = context.entity("cause") {
            damage_source = damage_source.with_causing_entity(cause);
        }
    }

    let Some(target_world) = target.level() else {
        return Err(CommandSyntaxError::dynamic(
            "The entity is not in a world or the world was dropped.",
        ));
    };

    if target.hurt(&target_world, &damage_source, amount) {
        context.source().send_success(
            &COMMANDS_DAMAGE_SUCCESS
                .message([
                    TextComponent::plain(format!("{amount:?}")),
                    target.display_name(),
                ])
                .component(),
            true,
        );
        Ok(1)
    } else {
        context
            .source()
            .send_failure(COMMANDS_DAMAGE_INVULNERABLE.msg().component());
        Ok(0)
    }
}
