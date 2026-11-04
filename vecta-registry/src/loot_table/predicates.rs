//! Vanilla's three code-bootstrapped loot predicates.
//!
//! Java registers them in `LootPredicates.bootstrap` rather than loading JSON, so
//! Vecta mirrors that structure instead of inventing a data-driven predicate
//! registry. Keys outside this table are an error, never a permissive default.

use vecta_utils::Identifier;

use super::LootCondition;
use super::conditions::ToolPredicate;

/// `minecraft:block/fast_cooking` — `MatchBlock` over smoker and blast furnace.
static FAST_COOKING_BLOCKS: &[LootCondition] = &[
    LootCondition::MatchBlock {
        block: Identifier::new_static("minecraft", "smoker"),
        properties: &[],
    },
    LootCondition::MatchBlock {
        block: Identifier::new_static("minecraft", "blast_furnace"),
        properties: &[],
    },
];

/// `minecraft:block/fast_cooking`.
pub static FAST_COOKING: LootCondition = LootCondition::AnyOf(FAST_COOKING_BLOCKS);

/// `minecraft:tool/can_shear` — `MatchTool` for shears.
pub static TOOL_CAN_SHEAR: LootCondition = LootCondition::MatchTool(ToolPredicate::Item(
    Identifier::new_static("minecraft", "shears"),
));

/// `minecraft:tool/can_silk_touch` — `MatchTool` with silk touch.
///
/// Vanilla expresses the enchantment as a data-component predicate; Vecta's
/// `ToolPredicate::HasEnchantment` is the existing equivalent check on the tool.
pub static TOOL_CAN_SILK_TOUCH: LootCondition =
    LootCondition::MatchTool(ToolPredicate::HasEnchantment {
        enchantment: Identifier::new_static("minecraft", "silk_touch"),
        min_level: 1,
    });

/// Resolves one of the code-bootstrapped predicates.
#[must_use]
pub fn by_key(key: &Identifier) -> Option<&'static LootCondition> {
    if key.namespace != "minecraft" {
        return None;
    }
    match key.path.as_ref() {
        "block/fast_cooking" => Some(&FAST_COOKING),
        "tool/can_shear" => Some(&TOOL_CAN_SHEAR),
        "tool/can_silk_touch" => Some(&TOOL_CAN_SILK_TOUCH),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_predicate_keys_resolve_and_unknown_keys_do_not() {
        assert!(by_key(&Identifier::vanilla_static("block/fast_cooking")).is_some());
        assert!(by_key(&Identifier::vanilla_static("tool/can_shear")).is_some());
        assert!(by_key(&Identifier::vanilla_static("tool/can_silk_touch")).is_some());
        assert!(by_key(&Identifier::vanilla_static("block/missing")).is_none());
        assert!(by_key(&Identifier::new_static("other", "block/fast_cooking")).is_none());
    }
}
