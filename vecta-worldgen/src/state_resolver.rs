use vecta_registry::blocks::BlockRef;
use vecta_registry::feature;
use vecta_registry::shared_structs;
use vecta_registry::{Registry, RegistryExt};
use vecta_utils::BlockStateId;

/// Resolves vanilla JSON/NBT block-state data to Vecta block-state ids.
pub struct WorldgenStateResolver;

impl WorldgenStateResolver {
    /// Resolves a block state from data.
    ///
    /// # Panics
    /// Panics if the block is not in the registry or if the state properties are invalid.
    #[must_use]
    pub fn block_state_from_data(
        registry: &Registry,
        data: &shared_structs::BlockStateData,
        context: &str,
    ) -> BlockStateId {
        let Some(block) = registry.blocks.by_key(&data.name) else {
            panic!("{context} references unknown block {}", data.name);
        };
        Self::block_state_from_parts(
            registry,
            block,
            &data.name,
            data.properties
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
            context,
        )
    }

    /// Resolves a feature block state from data.
    ///
    /// # Panics
    /// Panics if the state properties are invalid.
    #[must_use]
    pub fn feature_block_state_from_data(
        registry: &Registry,
        data: &feature::BlockStateData,
        context: &str,
    ) -> BlockStateId {
        Self::block_state_from_parts(
            registry,
            data.block,
            &data.block.key,
            data.properties.iter().copied(),
            context,
        )
    }

    fn block_state_from_parts<'a>(
        registry: &Registry,
        block: BlockRef,
        block_name: &vecta_utils::Identifier,
        data_properties: impl IntoIterator<Item = (&'a str, &'a str)>,
        context: &str,
    ) -> BlockStateId {
        let Some(state) = registry
            .blocks
            .state_id_from_block_defaulted_properties(block, data_properties)
        else {
            panic!("{context} references unknown or invalid state {block_name}");
        };
        state
    }
}
