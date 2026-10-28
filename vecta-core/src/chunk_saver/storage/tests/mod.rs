use std::slice;

use super::*;
use std::sync::Arc;

use crate::bootstrap::init_globals;
use crate::entity::{
    DEFAULT_MAX_AIR_SUPPLY, Entity, SharedEntity, entities::EndCrystalEntity, next_entity_id,
};
use crate::test_support::{TestEntity, fresh_test_world, insert_ready_full_chunk};
use glam::DVec3;
use rustc_hash::FxHashMap;
use text_components::TextComponent;
use vecta_registry::init_vanilla_registry;
use vecta_registry::vanilla_block_entity_types;
use vecta_registry::vanilla_blocks;
use vecta_registry::vanilla_entities;
use vecta_registry::vanilla_fluids;
use vecta_utils::BoundingBox;
use vecta_utils::types::UpdateFlags;
use vecta_worldgen::structure::StructureReferenceSet;

fn test_structure_piece() -> StructurePiece {
    StructurePiece {
        piece_type: Identifier::new_static("minecraft", "mscorridor"),
        bounding_box: BoundingBox::new(IVec3::new(0, 64, 0), IVec3::new(1, 65, 1)),
        gen_depth: 0,
        orientation: None,
        payload: StructurePiecePayload::Procedural(ProceduralPieceData::Unimplemented),
        ground_level_delta: 0,
        junctions: Vec::new(),
        projection: None,
    }
}

fn single_empty_section() -> Sections {
    Sections::from_owned(vec![ChunkSection::new_empty()].into_boxed_slice())
}

fn visible_homogeneous_value(section: Option<&LightSection>) -> Option<u8> {
    let Some(LightSection::Visible(LightSectionData::Homogeneous(value))) = section else {
        return None;
    };
    Some(*value)
}

mod chunks_sections;
mod entities;
mod light;
mod structures;
mod ticks;
