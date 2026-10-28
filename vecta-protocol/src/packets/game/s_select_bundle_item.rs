use std::io::{Cursor, Error, ErrorKind, Result};

use vecta_macros::ServerPacket;
use vecta_utils::{codec::VarInt, serial::ReadFrom};

/// Selects the highlighted item inside a bundle held in a menu slot.
///
/// `selected_item_index` is `-1` when the client clears the selection.
#[derive(ServerPacket, Debug, Clone, PartialEq, Eq)]
pub struct SSelectBundleItem {
    pub slot_id: i32,
    pub selected_item_index: i32,
}

impl ReadFrom for SSelectBundleItem {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let slot_id = VarInt::read(data)?.0;
        let selected_item_index = VarInt::read(data)?.0;

        if selected_item_index < -1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid bundle item index: {selected_item_index}"),
            ));
        }

        Ok(Self {
            slot_id,
            selected_item_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use vecta_utils::serial::WriteTo;

    use super::*;

    #[test]
    fn reads_slot_and_selection() {
        let mut bytes = Vec::new();
        VarInt(3).write(&mut bytes).unwrap();
        VarInt(1).write(&mut bytes).unwrap();

        let packet = SSelectBundleItem::read(&mut Cursor::new(bytes.as_slice())).unwrap();

        assert_eq!(
            packet,
            SSelectBundleItem {
                slot_id: 3,
                selected_item_index: 1,
            }
        );
    }

    #[test]
    fn rejects_indexes_below_the_cleared_selection() {
        let mut bytes = Vec::new();
        VarInt(0).write(&mut bytes).unwrap();
        VarInt(-2).write(&mut bytes).unwrap();

        assert!(SSelectBundleItem::read(&mut Cursor::new(bytes.as_slice())).is_err());
    }
}
