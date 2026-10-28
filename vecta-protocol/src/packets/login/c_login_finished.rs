use uuid::Uuid;
use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::GameProfile;
use vecta_registry::packets::login::C_LOGIN_FINISHED;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Login = C_LOGIN_FINISHED)]
pub struct CLoginFinished<'a> {
    pub game_profile: &'a GameProfile,
    pub session_id: Uuid,
}

impl<'a> CLoginFinished<'a> {
    #[must_use]
    pub const fn new(game_profile: &'a GameProfile, session_id: Uuid) -> Self {
        Self {
            game_profile,
            session_id,
        }
    }
}
