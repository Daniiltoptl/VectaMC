use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_SET_HELD_SLOT;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SET_HELD_SLOT)]
pub struct CSetHeldSlot {
    #[write(as = VarInt)]
    pub slot: i32,
}
