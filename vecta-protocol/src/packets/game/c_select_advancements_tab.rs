use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_SELECT_ADVANCEMENTS_TAB;
use vecta_utils::Identifier;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SELECT_ADVANCEMENTS_TAB)]
pub struct CSelectAdvancementsTab {
    tab: Option<Identifier>,
}

impl CSelectAdvancementsTab {
    #[must_use]
    pub const fn new(tab: Option<Identifier>) -> Self {
        Self { tab }
    }
}
