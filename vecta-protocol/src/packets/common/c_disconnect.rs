use text_components::TextComponent;
use text_components::resolving::TextResolutor;
use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::config::C_DISCONNECT;
use vecta_registry::packets::play::C_DISCONNECT as PLAY_C_DISCONNECT;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = C_DISCONNECT, Play = PLAY_C_DISCONNECT)]
pub struct CDisconnect {
    pub reason: TextComponent,
}

impl CDisconnect {
    #[must_use]
    pub fn new<T: TextResolutor>(reason: &TextComponent, player: &T) -> Self {
        Self {
            reason: reason.resolve(player),
        }
    }
}
