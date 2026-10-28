use text_components::TextComponent;
use vecta_macros::{ClientPacket, WriteTo};
use vecta_registry::packets::play::C_SET_ACTION_BAR_TEXT;

/// Sets the action-bar text displayed by the client.
#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = C_SET_ACTION_BAR_TEXT)]
pub struct CSetActionBarText {
    /// Text to display in the action bar.
    pub text: TextComponent,
}

impl CSetActionBarText {
    /// Creates an action-bar-text packet.
    #[must_use]
    pub fn new(text: impl Into<TextComponent>) -> Self {
        Self { text: text.into() }
    }
}
