use text_components::{TextComponent, resolving::TextResolutor};
use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::login::C_LOGIN_DISCONNECT;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = C_LOGIN_DISCONNECT)]
pub struct CLoginDisconnect {
    #[write(as = Json)]
    pub reason: TextComponent,
}

impl CLoginDisconnect {
    #[must_use]
    pub fn new<T: TextResolutor>(reason: &TextComponent, resolutor: &T) -> Self {
        Self {
            reason: reason.resolve(resolutor),
        }
    }
}
