use super::*;
use crate::item_stack::ItemStack;
use crate::loot_table::{EntityRef, ExtensionValue, FloatValue};
use crate::loot_table::{
    EntityRefFlags, LootCondition, LootContext, NumberProvider, loot_context_params as params,
};
use crate::{init_vanilla_registry, vanilla_blocks, vanilla_context_key_sets as sets};
use std::ptr;
use vecta_utils::random::{RandomSource, legacy_random::LegacyRandom};
use vecta_utils::{DowncastType, DowncastTypeKey, Identifier};

fn entity_ref(key: &Identifier) -> EntityRef<'_> {
    EntityRef {
        entity_type: Some(key),
        score_holder: None,
        flags: EntityRefFlags::default(),
        equipment: None,
        custom_name: None,
        sheep_color: None,
        sheep_sheared: None,
        chicken_variant: None,
    }
}

#[test]
fn wrong_value_family_cannot_overwrite_or_satisfy_a_required_key() {
    init_vanilla_registry();
    const WRONG_TOOL: ContextKey<FloatValue> = ContextKey::new("minecraft:tool");
    let tool = ItemStack::empty();
    let mut map = ContextMap::default();
    map.set(params::ORIGIN, Some((0.5, 64.5, 0.5)));
    map.set(
        params::BLOCK_STATE,
        Some(vanilla_blocks::STONE.default_state()),
    );
    map.set(WRONG_TOOL, Some(2.0));
    assert!(map.get(params::TOOL).is_none());
    assert_eq!(
        map.validate(&sets::BLOCK),
        Err(ContextMapError::NotAllowed(vec![WRONG_TOOL.id()]))
    );
    map.set(WRONG_TOOL, None);
    assert_eq!(
        map.validate(&sets::BLOCK),
        Err(ContextMapError::Missing(vec![params::TOOL.id()]))
    );
    map.set(params::TOOL, Some(&tool));
    assert!(ptr::eq(
        map.get_required(params::TOOL).expect("tool"),
        ptr::from_ref(&tool)
    ));
    assert!(map.validate(&sets::BLOCK).is_ok());
}

#[test]
fn optional_removal_is_distinct_from_zero_and_chest_uses_this_entity() {
    let player_key = Identifier::vanilla_static("player");
    let player = entity_ref(&player_key);
    let mut map = ContextMap::default();
    map.set(params::EXPLOSION_RADIUS, Some(0.0));
    assert_eq!(
        map.get_required(params::EXPLOSION_RADIUS).map(f32::to_bits),
        Ok(0.0_f32.to_bits())
    );
    map.set(params::EXPLOSION_RADIUS, None);
    assert!(matches!(
        map.get_required(params::EXPLOSION_RADIUS),
        Err(ContextMapError::Missing(_))
    ));
    map.set(params::ORIGIN, Some((0.5, 64.5, 0.5)));
    map.set(params::INTERACTING_ENTITY, Some(player));
    assert_eq!(
        map.validate(&sets::CHEST),
        Err(ContextMapError::NotAllowed(vec![
            params::INTERACTING_ENTITY.id()
        ]))
    );
    map.set(params::INTERACTING_ENTITY, None);
    map.set(params::THIS_ENTITY, Some(player));
    assert!(map.validate(&sets::CHEST).is_ok());
}

struct ExtensionData(String);

// SAFETY: This owner-scoped test key uniquely identifies ExtensionData.
unsafe impl DowncastType for ExtensionData {
    const TYPE_KEY: DowncastTypeKey =
        DowncastTypeKey::new("vecta:test/loot_context/extension_data");
}

#[test]
fn extensions_borrow_owner_values_without_type_id_or_snapshot_copies() {
    const KEY: ContextKey<ExtensionValue<ExtensionData>> = ContextKey::new("example:loot_value");
    const OTHER: ContextKey<FloatValue> = ContextKey::new("example:loot_value");
    let value = ExtensionData("runtime value".to_owned());
    let mut map = ContextMap::default();
    map.set(KEY, Some(&value));
    map.set(OTHER, Some(2.0));
    let result = map.get_required(KEY).expect("extension value");
    assert!(ptr::eq(result, ptr::from_ref(&value)));
    assert_eq!(result.0, "runtime value");
    map.set(OTHER, None);
    assert!(map.has(KEY.id()));
    assert!(!map.has(OTHER.id()));
}

#[test]
fn killed_by_player_depends_on_last_damage_player_presence() {
    let key = Identifier::vanilla_static("player");
    let player = entity_ref(&key);
    let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(42));
    let mut context = LootContext::new(&mut rng)
        .with_parameter(params::THIS_ENTITY, player)
        .with_parameter(params::ATTACKING_ENTITY, player);
    assert!(!LootCondition::KilledByPlayer.test(&mut context));
    context.params.set(params::LAST_DAMAGE_PLAYER, Some(player));
    assert!(LootCondition::KilledByPlayer.test(&mut context));
    context.params.set(params::LAST_DAMAGE_PLAYER, None);
    assert!(!LootCondition::KilledByPlayer.test(&mut context));
}

#[test]
fn parameter_reads_and_validation_preserve_the_shared_seeded_stream() {
    let tool = ItemStack::empty();
    let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(123));
    let mut expected = RandomSource::Legacy(LegacyRandom::from_seed(123));
    let mut context = LootContext::new(&mut rng)
        .with_parameter(params::ORIGIN, (0.5, 64.5, 0.5))
        .with_parameter(params::TOOL, &tool);
    let provider = NumberProvider::Uniform {
        min: -3.0,
        max: 5.0,
    };
    for invalid in [false, true, false] {
        context
            .params
            .set(params::EXPLOSION_RADIUS, invalid.then_some(0.0));
        assert_eq!(context.params.validate(&sets::FISHING).is_err(), invalid);
        assert!(context.params.get(params::TOOL).is_some());
        assert_eq!(
            provider.get_float(&mut context).to_bits(),
            provider.get_simple(&mut expected).to_bits()
        );
    }
}
