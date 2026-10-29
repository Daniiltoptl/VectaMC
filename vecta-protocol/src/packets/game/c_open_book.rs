//! Clientbound packet that opens a written book screen.

use std::io::{Result, Write};

use vecta_macros::ClientPacket;
use vecta_registry::packets::play::C_OPEN_BOOK;
use vecta_utils::{serial::WriteTo, types::InteractionHand};

/// Opens the book view screen for the stack held in `hand`.
///
/// The server only sends this for stacks carrying written-book content; the
/// client opens the writable book editor locally, so there is no serverbound
/// equivalent.
#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_OPEN_BOOK)]
pub struct COpenBook {
    /// Hand holding the book the client should display.
    pub hand: InteractionHand,
}

impl WriteTo for COpenBook {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.hand.write(writer)
    }
}

#[cfg(test)]
mod tests {
    use vecta_utils::{
        codec::VarInt,
        serial::{ReadFrom as _, WriteTo as _},
        types::InteractionHand,
    };

    use super::COpenBook;

    #[test]
    fn writes_the_hand_id_as_a_var_int() {
        for (hand, id) in [
            (InteractionHand::MainHand, 0),
            (InteractionHand::OffHand, 1),
        ] {
            let mut bytes = Vec::new();
            COpenBook { hand }
                .write(&mut bytes)
                .expect("open book packet should encode");

            assert_eq!(
                InteractionHand::read(&mut std::io::Cursor::new(bytes.as_slice())).unwrap(),
                hand
            );
            assert_eq!(bytes, vec![u8::try_from(id).unwrap()]);
            assert_eq!(VarInt(id).0, id);
        }
    }
}
