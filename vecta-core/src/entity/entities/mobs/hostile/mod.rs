//! Hostile entity implementations.
//!
//! Those mobs are aggressive creatures that attack players on sight.

/// Hostile creeper entity module.
pub mod creeper;
/// Hostile endermite entity module.
pub mod endermite;
/// Hostile spider entity module.
pub mod spider;

pub use creeper::CreeperEntity;
pub use endermite::EndermiteEntity;
pub use spider::{CaveSpiderEntity, SpiderEntity};
