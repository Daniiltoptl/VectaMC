use vecta_macros::{ReadFrom, ServerPacket};
use vecta_utils::types::GameType;

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SChangeGameMode {
    pub gamemode: GameType,
}
