use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_BLOCK_UPDATE;
use vecta_utils::{BlockPos, BlockStateId};

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_BLOCK_UPDATE)]
pub struct CBlockUpdate {
    pub pos: BlockPos,
    pub block_state: BlockStateId,
}
