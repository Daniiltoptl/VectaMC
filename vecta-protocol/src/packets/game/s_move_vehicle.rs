use glam::DVec3;
use vecta_macros::{ReadFrom, ServerPacket};

/// Serverbound controlled-vehicle movement packet.
#[derive(ReadFrom, Clone, Debug, ServerPacket)]
pub struct SMoveVehicle {
    pub pos: DVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}
