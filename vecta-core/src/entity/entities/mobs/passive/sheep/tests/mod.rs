use std::io::Cursor;

use simdnbt::borrow::read_compound as read_borrowed_compound;
use vecta_registry::{
    init_vanilla_registry, vanilla_attributes, vanilla_damage_types, vanilla_entities,
    vanilla_items,
};
use vecta_utils::types::InteractionHand;

use crate::entity::damage::DamageSource;
use crate::entity::init_entities;
use crate::entity::{SharedEntity, next_entity_id};
use crate::test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk};
use vecta_utils::ChunkPos;

use super::*;

mod core;
mod persistence;
