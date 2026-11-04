pub use crate::{DyeColor, equipment::EquipmentSlotGroup};
use crate::{
    REGISTRY, RegistryExt, TaggedRegistryExt, blocks::block_state_ext::BlockStateExt,
    instrument::InstrumentRef, item_stack::ItemStack,
};
use rustc_hash::FxHashMap;
use vecta_utils::Identifier;

mod conditions;
mod context;
mod entries;
mod functions;
mod number;
mod registry;
#[path = "predicates.rs"]
pub mod vanilla_loot_predicates;

pub use conditions::*;
pub use context::*;
pub use entries::*;
pub use functions::*;
pub use number::*;
pub use registry::*;

#[cfg(test)]
mod tests;
