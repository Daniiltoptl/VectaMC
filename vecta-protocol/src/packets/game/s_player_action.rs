use vecta_macros::{ReadFrom, ServerPacket};
use vecta_registry::blocks::properties::Direction;
use vecta_utils::BlockPos;

/// Action types for the player action packet.
#[derive(ReadFrom, Clone, Copy, Debug, PartialEq, Eq)]
#[read(as = VarInt)]
pub enum PlayerAction {
    StartDestroyBlock = 0,
    ChangeDestroyDirection = 1,
    AbortDestroyBlock = 2,
    StopDestroyBlock = 3,
    DropAllItems = 4,
    DropItem = 5,
    ReleaseUseItem = 6,
    SwapItemWithOffhand = 7,
    Stab = 8,
}

/// Serverbound packet sent when a player performs an action like mining a block.
#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SPlayerAction {
    pub action: PlayerAction,
    pub pos: BlockPos,
    pub direction: Direction,
    #[read(as = VarInt)]
    pub sequence: i32,
}
