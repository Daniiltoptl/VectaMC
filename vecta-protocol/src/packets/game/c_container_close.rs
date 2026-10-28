use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_CONTAINER_CLOSE;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_CONTAINER_CLOSE)]
pub struct CContainerClose {
    #[write(as = VarInt)]
    pub container_id: i32,
}
