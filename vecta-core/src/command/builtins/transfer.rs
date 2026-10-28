use std::sync::Arc;

use text_components::TextComponent;
use vecta_protocol::packets::common::CTransfer;
use vecta_utils::{Identifier, translations};

use crate::{
    command::{
        brigadier::{ArgumentType, CommandContext, CommandNodeBuilder, CommandSyntaxError},
        execution::{CommandSource, VectaArgumentType, VectaCommandRuntime, argument, literal},
        registration::CommandRegistration,
    },
    entity::Entity,
    player::Player,
};

pub(super) fn registration() -> CommandRegistration<CommandSource> {
    CommandRegistration::new(Identifier::vanilla_static("transfer"), |_| command())
}

fn command() -> CommandNodeBuilder<CommandSource, VectaCommandRuntime> {
    literal("transfer").then(
        argument("hostname", ArgumentType::string())
            .executes(|c| {
                transfer(
                    c.source(),
                    c.string("hostname")?,
                    25565,
                    &[player_executor(c)?],
                )
            })
            .then(
                argument("port", ArgumentType::integer(1, 65535))
                    .executes(|c| {
                        transfer(
                            c.source(),
                            c.string("hostname")?,
                            c.integer("port")?,
                            &[player_executor(c)?],
                        )
                    })
                    .then(
                        argument("players", VectaArgumentType::players()).executes(|c| {
                            transfer(
                                c.source(),
                                c.string("hostname")?,
                                c.integer("port")?,
                                &c.players("players")?,
                            )
                        }),
                    ),
            ),
    )
}

fn transfer(
    source: &CommandSource,
    hostname: &str,
    port: i32,
    players: &[Arc<Player>],
) -> Result<i32, CommandSyntaxError> {
    if players.is_empty() {
        return Err(CommandSyntaxError::dynamic(
            translations::COMMANDS_TRANSFER_ERROR_NO_PLAYERS.msg(),
        ));
    }

    for p in players {
        p.send_packet(CTransfer::new(hostname, port));
    }

    if players.len() == 1 {
        source.send_success(
            &TextComponent::translated(translations::COMMANDS_TRANSFER_SUCCESS_SINGLE.message([
                players[0].display_name(),
                TextComponent::from(hostname.to_string()),
                TextComponent::from(port.to_string()),
            ])),
            true,
        );
    } else {
        source.send_success(
            &TextComponent::translated(translations::COMMANDS_TRANSFER_SUCCESS_MULTIPLE.message([
                TextComponent::from(players.len().to_string()),
                TextComponent::from(hostname.to_string()),
                TextComponent::from(port.to_string()),
            ])),
            true,
        );
    }

    Ok(players.len() as i32)
}

fn player_executor(
    c: &CommandContext<CommandSource, VectaCommandRuntime>,
) -> Result<Arc<Player>, CommandSyntaxError> {
    c.source()
        .player()
        .ok_or(CommandSyntaxError::dynamic(
            translations::PERMISSIONS_REQUIRES_PLAYER.msg(),
        ))
        .cloned()
}
