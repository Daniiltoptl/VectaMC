use uuid::Uuid;
use vecta_macros::{ReadFrom, ServerPacket};

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SHello {
    #[read(as = Prefixed(VarInt), bound = 16)]
    pub name: String,
    pub profile_id: Uuid,
}
