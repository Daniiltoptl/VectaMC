//! Vanilla raw component messaging command.

use vecta_protocol::packets::game::CSystemChat;
use vecta_utils::Identifier;

use super::super::{
    brigadier::{CommandNodeBuilder, CommandSyntaxError},
    execution::{
        CommandSource, CommandTextResolver, VectaArgumentType, VectaCommandContext,
        VectaCommandRuntime, argument, literal,
    },
    registration::CommandRegistration,
};

pub(super) fn registration() -> CommandRegistration<CommandSource> {
    CommandRegistration::new(Identifier::vanilla_static("tellraw"), |_| command())
}

fn command() -> CommandNodeBuilder<CommandSource, VectaCommandRuntime> {
    literal("tellraw").then(
        argument("targets", VectaArgumentType::players())
            .then(argument("message", VectaArgumentType::component()).executes(send_message)),
    )
}

fn send_message(context: &VectaCommandContext<CommandSource>) -> Result<i32, CommandSyntaxError> {
    let targets = context.players("targets")?;
    let message = context.text_component("message")?;
    let result = i32::try_from(targets.len()).map_err(|_| {
        CommandSyntaxError::dynamic("Target player count exceeds the command result range")
    })?;

    for target in targets {
        let message = message.try_resolve(&CommandTextResolver::with_entity_override(
            context.source(),
            target.as_ref(),
        ))?;
        target.send_packet(CSystemChat {
            content: message,
            overlay: false,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use vecta_registry::init_vanilla_registry;

    use super::super::create_dispatcher;
    use crate::command::execution::VectaArgumentType;

    #[test]
    fn tellraw_graph_uses_player_targets_and_component_messages() {
        init_vanilla_registry();
        let Ok(dispatcher) = create_dispatcher() else {
            panic!("built-in commands should register");
        };
        let Some(tellraw) = dispatcher.children(dispatcher.root()).and_then(|children| {
            children.iter().copied().find(|child| {
                dispatcher
                    .node(*child)
                    .is_some_and(|node| node.name() == "tellraw")
            })
        }) else {
            panic!("tellraw root should exist");
        };
        let Some(root) = dispatcher.node(tellraw) else {
            panic!("tellraw root node should exist");
        };
        assert!(root.is_restricted());
        assert!(!root.is_executable());

        let Some(targets) = dispatcher
            .children(tellraw)
            .and_then(|children| children.first())
            .copied()
        else {
            panic!("tellraw targets should exist");
        };
        assert_eq!(
            dispatcher
                .node(targets)
                .and_then(|node| node.argument_type()),
            Some(&VectaArgumentType::players())
        );

        let Some(message) = dispatcher
            .children(targets)
            .and_then(|children| children.first())
            .copied()
        else {
            panic!("tellraw message should exist");
        };
        let Some(message) = dispatcher.node(message) else {
            panic!("tellraw message node should exist");
        };
        assert_eq!(
            message.argument_type(),
            Some(&VectaArgumentType::component())
        );
        assert!(message.is_executable());
    }
}
