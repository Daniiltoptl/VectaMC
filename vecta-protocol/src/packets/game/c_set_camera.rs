use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_SET_CAMERA;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SET_CAMERA)]
pub struct CSetCamera {
    #[write(as = VarInt)]
    pub camera_id: i32,
}
