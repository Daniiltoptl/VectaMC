use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::config::C_KEEP_ALIVE;
use vecta_registry::packets::play::C_KEEP_ALIVE as PLAY_C_KEEP_ALIVE;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = C_KEEP_ALIVE, Play = PLAY_C_KEEP_ALIVE)]
pub struct CKeepAlive {
    pub id: i64,
}

impl CKeepAlive {
    #[must_use]
    pub const fn new(id: i64) -> Self {
        Self { id }
    }
}
