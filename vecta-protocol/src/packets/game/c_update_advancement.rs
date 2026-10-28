use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::advancement::AdvancementProgressData;
use vecta_registry::advancement::registry::AdvancementNodeRef;
use vecta_registry::packets::play::C_UPDATE_ADVANCEMENTS;
use vecta_utils::Identifier;

/// Updates the client advancement tree and progress
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_UPDATE_ADVANCEMENTS)]
pub struct CUpdateAdvancements {
    /// The number of ticks to step forward.
    pub reset: bool,
    pub added: Vec<AdvancementNodeRef>,
    pub removed: Vec<Identifier>,
    pub progress: Vec<AdvancementProgressData>,
    pub show_advancements: bool,
}

impl CUpdateAdvancements {
    /// Creates a new update advancement packet.
    #[must_use]
    pub const fn new(
        reset: bool,
        added: Vec<AdvancementNodeRef>,
        progress: Vec<AdvancementProgressData>,
        removed: Vec<Identifier>,
        show_advancements: bool,
    ) -> Self {
        Self {
            reset,
            added,
            removed,
            progress,
            show_advancements,
        }
    }
}
