use std::io::Write;
use vecta_macros::ClientPacket;
use vecta_registry::packets::play::C_AWARD_STATS;
use vecta_registry::stat::Stat;
use vecta_utils::codec::VarInt;
use vecta_utils::serial::WriteTo;

/// Clientbound packet that updates the client with the stats that have been
/// marked dirty.
#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_AWARD_STATS)]
pub struct CAwardStats {
    /// The dirty stats to send.
    pub stats: Vec<(Stat, i32)>,
}

impl WriteTo for CAwardStats {
    fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        VarInt(self.stats.len() as i32).write(writer)?;
        for (stat, count) in &self.stats {
            stat.write(writer)?;
            VarInt(*count).write(writer)?;
        }
        Ok(())
    }
}
