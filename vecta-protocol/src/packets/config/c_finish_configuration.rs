use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::config::C_FINISH_CONFIGURATION;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = C_FINISH_CONFIGURATION)]
pub struct CFinishConfiguration {}
