use vecta_macros::{ReadFrom, ServerPacket};

#[derive(ServerPacket, ReadFrom, Clone, Debug)]
pub struct SSetCarriedItem {
    pub slot: i16,
}
