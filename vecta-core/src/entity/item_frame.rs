//! Shared item-frame entity behavior.

use vecta_utils::Direction;

use super::Entity;

/// Vanilla `ItemFrame` behavior shared by item frames and their glow subtype.
pub trait ItemFrame: Entity {
    /// Returns the direction from the backing block toward the frame.
    fn direction(&self) -> Direction;

    /// Returns the comparator signal strength for this frame: 0 when empty,
    /// otherwise derived from the framed item's rotation.
    fn analog_output(&self) -> i32;

    /// Plays this frame variant's placement sound.
    fn play_placement_sound(&self);
}
