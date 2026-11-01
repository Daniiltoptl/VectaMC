use super::*;
use vecta_utils::random::Random as _;

const RANDOM_COUNT: ConditionalLootFunction = ConditionalLootFunction {
    conditions: &[],
    function: LootFunction::SetCount {
        count: NumberProvider::Uniform {
            min: 1.0,
            max: 20.0,
        },
        add: false,
    },
};

const WEIGHTED_ITEMS: &[LootEntry] = &[
    LootEntry::Item {
        name: Identifier::vanilla_static("stone"),
        weight: 1,
        quality: 0,
        conditions: &[],
        functions: &[],
    },
    LootEntry::Item {
        name: Identifier::vanilla_static("diamond"),
        weight: 1,
        quality: 0,
        conditions: &[],
        functions: &[],
    },
];

fn evaluate(
    pools: &'static [LootPool],
    functions: &'static [ConditionalLootFunction],
) -> Vec<ItemStack> {
    init_test_registries();
    let table = LootTable {
        key: Identifier::new_static("vecta", "test/evaluation_order"),
        loot_type: LootType::Chest,
        pools,
        functions,
        random_sequence: None,
    };
    let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(42));
    table.get_random_items(&mut LootContext::new(&mut rng))
}

fn assert_interleaved_rolls(items: &[ItemStack]) {
    // Java Random(42): nextInt(2), nextInt(20)+1 for each emitted stack.
    let actual: Vec<_> = items
        .iter()
        .map(|item| (item.is(&vanilla_items::DIAMOND), item.count()))
        .collect();
    assert_eq!(actual, [(true, 4), (true, 5), (false, 6)]);
}

#[test]
fn table_functions_run_before_the_next_weighted_roll() {
    let items = evaluate(
        const {
            &[LootPool {
                rolls: NumberProvider::Constant(3.0),
                bonus_rolls: 0.0,
                entries: WEIGHTED_ITEMS,
                conditions: &[],
                functions: &[],
            }]
        },
        const { &[RANDOM_COUNT] },
    );
    assert_interleaved_rolls(&items);
}

#[test]
fn pool_functions_run_before_the_next_weighted_roll() {
    let items = evaluate(
        const {
            &[LootPool {
                rolls: NumberProvider::Constant(3.0),
                bonus_rolls: 0.0,
                entries: WEIGHTED_ITEMS,
                conditions: &[],
                functions: &[RANDOM_COUNT],
            }]
        },
        &[],
    );
    assert_interleaved_rolls(&items);
}

#[test]
fn outer_functions_interleave_with_nested_table_generation() {
    let items = evaluate(
        const {
            &[LootPool {
                rolls: NumberProvider::Constant(1.0),
                bonus_rolls: 0.0,
                entries: &[LootEntry::InlineLootTable {
                    pools: &[LootPool {
                        rolls: NumberProvider::Constant(2.0),
                        bonus_rolls: 0.0,
                        entries: &[LootEntry::Item {
                            name: Identifier::vanilla_static("stone"),
                            weight: 1,
                            quality: 0,
                            conditions: &[],
                            functions: &[RANDOM_COUNT],
                        }],
                        conditions: &[],
                        functions: &[],
                    }],
                    weight: 1,
                    quality: 0,
                    conditions: &[],
                    functions: &[],
                }],
                conditions: &[],
                functions: &[],
            }]
        },
        const {
            &[ConditionalLootFunction {
                conditions: &[],
                function: LootFunction::SetCount {
                    count: NumberProvider::Uniform {
                        min: 1.0,
                        max: 20.0,
                    },
                    add: true,
                },
            }]
        },
    );
    // Java Random(42): inner nextInt(20)+1 then outer nextInt(20)+1 per item.
    assert_eq!(
        items.iter().map(ItemStack::count).collect::<Vec<_>>(),
        [15, 14]
    );
}

#[test]
fn stack_limits_are_applied_after_all_loot_functions() {
    let items = evaluate(
        const {
            &[LootPool {
                rolls: NumberProvider::Constant(1.0),
                bonus_rolls: 0.0,
                entries: &[LootEntry::Item {
                    name: Identifier::vanilla_static("stone"),
                    weight: 1,
                    quality: 0,
                    conditions: &[],
                    functions: &[],
                }],
                conditions: &[],
                functions: &[],
            }]
        },
        const {
            &[ConditionalLootFunction {
                conditions: &[],
                function: LootFunction::SetCount {
                    count: NumberProvider::Constant(129.0),
                    add: false,
                },
            }]
        },
    );
    assert_eq!(
        items.iter().map(ItemStack::count).collect::<Vec<_>>(),
        [64, 64, 1]
    );
}

#[test]
fn outer_functions_can_restore_a_zero_count_entry() {
    let items = evaluate(
        const {
            &[LootPool {
                rolls: NumberProvider::Constant(1.0),
                bonus_rolls: 0.0,
                entries: &[LootEntry::Item {
                    name: Identifier::vanilla_static("stone"),
                    weight: 1,
                    quality: 0,
                    conditions: &[],
                    functions: &[ConditionalLootFunction {
                        conditions: &[],
                        function: LootFunction::SetCount {
                            count: NumberProvider::Constant(0.0),
                            add: false,
                        },
                    }],
                }],
                conditions: &[],
                functions: &[],
            }]
        },
        const {
            &[ConditionalLootFunction {
                conditions: &[],
                function: LootFunction::SetCount {
                    count: NumberProvider::Constant(5.0),
                    add: false,
                },
            }]
        },
    );
    assert_eq!(items.iter().map(ItemStack::count).collect::<Vec<_>>(), [5]);
}

#[test]
fn zero_level_uniform_bonus_still_advances_the_loot_stream() {
    init_test_registries();
    let mut rng = RandomSource::Legacy(LegacyRandom::from_seed(42));
    let mut context = LootContext::new(&mut rng);
    let mut item = ItemStack::new(&vanilla_items::STONE);
    LootFunction::ApplyBonus {
        enchantment: Identifier::vanilla_static("fortune"),
        formula: BonusFormula::UniformBonusCount {
            bonus_multiplier: 1,
        },
    }
    .apply(&mut item, &mut context);
    assert_eq!(item.count(), 1);
    // java.util.Random(42).nextInt(1), then nextInt().
    assert_eq!(context.rng.next_i32(), 234_785_527);
}
