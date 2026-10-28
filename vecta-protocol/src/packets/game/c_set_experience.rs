use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_SET_EXPERIENCE;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SET_EXPERIENCE)]
pub struct CSetExperience {
    pub progress: f32,
    #[write(as = VarInt)]
    pub level: i32,
    #[write(as = VarInt)]
    pub total_experience: i32,
}
