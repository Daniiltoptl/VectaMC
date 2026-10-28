use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::str::FromStr;

use flate2::read::GzDecoder;
use glam::{DVec3, IVec3};
use simdnbt::borrow::{
    Nbt as BorrowedNbt, NbtCompound as BorrowedNbtCompound,
    NbtCompoundList as BorrowedNbtCompoundList, NbtList as BorrowedNbtList, read as read_nbt,
    read_compound as read_borrowed_compound,
};
use simdnbt::owned::{NbtCompound, NbtTag};
use text_components::TextComponent;
use uuid::Uuid;
use vecta_registry::block_entity_type::BlockEntityTypeRef;
use vecta_registry::blocks::properties::Direction as BlockPropertyDirection;
use vecta_registry::blocks::properties::{BlockStateProperties, Half};
use vecta_registry::blocks::{self};
use vecta_registry::blocks::{BlockRef, block_state_ext::BlockStateExt as _};
use vecta_registry::entity_type::EntityTypeRef;
use vecta_registry::fluid::FluidState;
use vecta_registry::shared_structs::BlockStateData;
use vecta_registry::structure::LiquidSettingsData;
use vecta_registry::structure_processor::{
    PosRuleTestData, ProcessorRuleData, RuleBlockEntityModifierData, StructureProcessorAxis,
    StructureProcessorKind, StructureRuleTestData,
};
use vecta_registry::template_pool::Projection;
use vecta_registry::vanilla_block_tags::BlockTag;
use vecta_registry::{
    Registry, RegistryExt, TaggedRegistryExt, vanilla_block_entity_types, vanilla_blocks,
    vanilla_template_pools,
};
use vecta_utils::random::legacy_random::LegacyRandom;
use vecta_utils::random::worldgen_random::WorldgenRandom;
use vecta_utils::random::{PositionalRandom, Random, RandomSource};
use vecta_utils::value_providers::IntProvider;
use vecta_utils::{
    BlockPos, BlockStateId, BoundingBox, Direction, Identifier, Rotation, types::UpdateFlags,
};

use crate::behavior::BLOCK_BEHAVIORS;
use crate::chunk::heightmap::HeightmapType;
use crate::entity::{
    DEFAULT_MAX_AIR_SUPPLY, ENTITIES, EntityBaseSaveData, EntityFireFreezeState, EntityLoadRequest,
    MAX_ENTITY_TAGS,
};
use crate::worldgen::region::WorldGenRegion;
use vecta_worldgen::state_resolver::WorldgenStateResolver;
use vecta_worldgen::structure::{StructureBlockIgnore, StructureMirror};

/// Loaded vanilla structure template payload.
///
/// Vecta keeps template data separate from template-pool metadata. Pools only need jigsaw
/// summaries during structure-start planning; feature and piece placement need the full NBT
/// block payload and processors, so this type mirrors vanilla's loaded `StructureTemplate`.
#[derive(Debug, Clone)]
pub(crate) struct StructureTemplate {
    size: IVec3,
    palettes: Vec<StructureTemplatePalette>,
    entities: Vec<StructureEntityInfo>,
}

#[derive(Debug, Clone)]
struct StructureTemplatePalette {
    blocks: Vec<StructureBlockInfo>,
}

#[derive(Debug, Clone)]
struct StructureBlockInfo {
    pos: BlockPos,
    state: BlockStateId,
    nbt: Option<NbtCompound>,
}

#[derive(Debug, Clone)]
struct StructureEntityInfo {
    pos: DVec3,
    block_pos: BlockPos,
    entity_type: EntityTypeRef,
    rotation: (f32, f32),
    velocity: DVec3,
    fall_distance: f64,
    fire_freeze: EntityFireFreezeState,
    on_ground: bool,
    save_data: EntityBaseSaveData,
    nbt: NbtCompound,
}

#[derive(Debug, Clone, PartialEq)]
struct ProcessedBlockInfo {
    template_pos: BlockPos,
    world_pos: BlockPos,
    state: BlockStateId,
    nbt: Option<NbtCompound>,
}

pub(crate) struct StructureDataMarker {
    pub(crate) metadata: String,
    pub(crate) pos: BlockPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructureProcessorRandom {
    /// Vanilla `StructurePlaceSettings.setRandom(random)`.
    Placement,
    /// Vanilla `StructurePlaceSettings.getRandom(pos)` fallback.
    Positional,
}

pub(crate) struct StructurePlaceSettings<'a> {
    pub(crate) mirror: StructureMirror,
    pub(crate) rotation: Rotation,
    pub(crate) rotation_pivot: BlockPos,
    pub(crate) bounding_box: BoundingBox,
    pub(crate) processors: &'a [StructureProcessorKind],
    pub(crate) block_ignore: StructureBlockIgnore,
    pub(crate) late_block_ignore: StructureBlockIgnore,
    pub(crate) replace_jigsaws: bool,
    pub(crate) projection: Option<Projection>,
    pub(crate) processor_random: StructureProcessorRandom,
    pub(crate) liquid_settings: LiquidSettingsData,
}

mod loading;
mod placement;
mod processors;
mod state_transforms;

#[cfg(test)]
mod tests;
