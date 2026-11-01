//! Dispenser-specific pieces.
//!
//! Vanilla keeps the dispense contract in `net.minecraft.core.dispenser`, one
//! class per file. The shared contract lives in [`dispense`]; the per-item
//! behaviors land here as later tasks add them.

pub(super) mod dispense;
