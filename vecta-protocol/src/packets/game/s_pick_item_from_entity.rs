use vecta_macros::{ReadFrom, ServerPacket};

/// Serverbound packet sent when a player uses the pick block key (middle click) on an entity.
#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SPickItemFromEntity {
    #[read(as = VarInt)]
    pub id: i32,
    pub include_data: bool,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use vecta_utils::serial::ReadFrom as _;

    use super::SPickItemFromEntity;

    #[test]
    fn pick_item_from_entity_reads_vanilla_field_order() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0xac, 0x02]); // VarInt 300
        bytes.push(1);

        let packet = SPickItemFromEntity::read(&mut Cursor::new(bytes.as_slice()))
            .expect("pick item from entity should decode");

        assert_eq!(packet.id, 300);
        assert!(packet.include_data);
    }

    #[test]
    fn pick_item_from_entity_accepts_an_absent_data_flag() {
        let bytes = vec![7, 0];

        let packet = SPickItemFromEntity::read(&mut Cursor::new(bytes.as_slice()))
            .expect("pick item from entity should decode");

        assert_eq!(packet.id, 7);
        assert!(!packet.include_data);
    }

    #[test]
    fn pick_item_from_entity_rejects_a_missing_data_flag() {
        let bytes = vec![7];

        assert!(SPickItemFromEntity::read(&mut Cursor::new(bytes.as_slice())).is_err());
    }
}
