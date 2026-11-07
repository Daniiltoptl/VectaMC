//! Arithmetic, RNG and parsing checks for the provider model.

use super::*;

/// A context with no parameters and a deterministic RNG.
fn test_context(rng: &mut vecta_utils::random::RandomSource) -> LootContext<'_> {
    LootContext::new(rng)
}

fn float_ctx(provider: &FloatProvider, rng: &mut vecta_utils::random::RandomSource) -> f32 {
    provider.get_float(&mut test_context(rng))
}

fn int_ctx(provider: &IntProvider, rng: &mut vecta_utils::random::RandomSource) -> i32 {
    provider.get_int(&mut test_context(rng))
}

fn seed() -> vecta_utils::random::RandomSource {
    use vecta_utils::random::{RandomSource, legacy_random::LegacyRandom};
    RandomSource::Legacy(LegacyRandom::from_seed(0x1234_5678))
}

#[test]
fn int_arithmetic_matches_java_edge_cases() {
    let mut rng = seed();
    // `Math.absExact(MIN)` and `Math.negateExact(MIN)` throw, and the safe
    // wrapper turns that into 0.
    assert_eq!(
        int_ctx(
            &IntProvider::Abs(ProviderRef::inline(IntProvider::Constant(i32::MIN))),
            &mut rng
        ),
        0
    );
    assert_eq!(
        int_ctx(
            &IntProvider::Negate(ProviderRef::inline(IntProvider::Constant(i32::MIN))),
            &mut rng
        ),
        0
    );
    // `Integer.MIN_VALUE / -1` overflows silently in Java; `%` gives 0.
    assert_eq!(
        int_ctx(
            &IntProvider::Div(
                ProviderRef::inline(IntProvider::Constant(i32::MIN)),
                ProviderRef::inline(IntProvider::Constant(-1))
            ),
            &mut rng
        ),
        i32::MIN
    );
    assert_eq!(
        int_ctx(
            &IntProvider::Mod(
                ProviderRef::inline(IntProvider::Constant(i32::MIN)),
                ProviderRef::inline(IntProvider::Constant(-1))
            ),
            &mut rng
        ),
        0
    );
    // `Math.floorDivExact(MIN, -1)` throws.
    assert_eq!(
        int_ctx(
            &IntProvider::FloorDiv(
                ProviderRef::inline(IntProvider::Constant(i32::MIN)),
                ProviderRef::inline(IntProvider::Constant(-1))
            ),
            &mut rng
        ),
        0
    );
    // Floor division and floor modulus round towards negative infinity.
    assert_eq!(
        int_ctx(
            &IntProvider::FloorDiv(
                ProviderRef::inline(IntProvider::Constant(-7)),
                ProviderRef::inline(IntProvider::Constant(2))
            ),
            &mut rng
        ),
        -4
    );
    assert_eq!(
        int_ctx(
            &IntProvider::FloorMod(
                ProviderRef::inline(IntProvider::Constant(-7)),
                ProviderRef::inline(IntProvider::Constant(2))
            ),
            &mut rng
        ),
        1
    );
    // `Math.subtractExact` throws on overflow.
    assert_eq!(
        int_ctx(
            &IntProvider::Sub(
                ProviderRef::inline(IntProvider::Constant(i32::MIN)),
                ProviderRef::inline(IntProvider::Constant(1))
            ),
            &mut rng
        ),
        0
    );
    // A negative divisor separates floor arithmetic from Euclidean arithmetic.
    for (left, right, quotient, modulus) in [(7, -2, -4, -1), (-7, -2, 3, -1), (6, -2, -3, 0)] {
        assert_eq!(
            int_ctx(
                &IntProvider::FloorDiv(
                    ProviderRef::inline(IntProvider::Constant(left)),
                    ProviderRef::inline(IntProvider::Constant(right))
                ),
                &mut rng
            ),
            quotient
        );
        assert_eq!(
            int_ctx(
                &IntProvider::FloorMod(
                    ProviderRef::inline(IntProvider::Constant(left)),
                    ProviderRef::inline(IntProvider::Constant(right))
                ),
                &mut rng
            ),
            modulus
        );
    }
    // Sum accumulates in a long and then checks the int range.
    assert_eq!(
        int_ctx(
            &IntProvider::Add(vec![
                ProviderRef::inline(IntProvider::Constant(i32::MAX)),
                ProviderRef::inline(IntProvider::Constant(1)),
            ]),
            &mut rng
        ),
        0
    );
    // `Math.powExact` rejects negative exponents and overflow.
    assert_eq!(
        int_ctx(
            &IntProvider::Pow {
                base: ProviderRef::inline(IntProvider::Constant(2)),
                exponent: ProviderRef::inline(IntProvider::Constant(-1)),
            },
            &mut rng
        ),
        0
    );
    assert_eq!(
        int_ctx(
            &IntProvider::Pow {
                base: ProviderRef::inline(IntProvider::Constant(-2)),
                exponent: ProviderRef::inline(IntProvider::Constant(31)),
            },
            &mut rng
        ),
        i32::MIN
    );
}

#[test]
fn float_arithmetic_matches_java_edge_cases() {
    let mut rng = seed();
    // `Mth.ceil` narrows through int, so NaN becomes 0 and huge values saturate.
    assert_eq!(
        float_ctx(
            &FloatProvider::Ceil(ProviderRef::inline(FloatProvider::Constant(f32::NAN))),
            &mut rng
        ),
        0.0
    );
    assert_eq!(
        float_ctx(
            &FloatProvider::Ceil(ProviderRef::inline(FloatProvider::Constant(1.0e10))),
            &mut rng
        ),
        i32::MAX as f32
    );
    // `Math.round`: NaN is 0, 2.5 rounds up, -2.5 rounds towards positive.
    assert_eq!(
        float_ctx(
            &FloatProvider::Round(ProviderRef::inline(FloatProvider::Constant(2.5))),
            &mut rng
        ),
        3.0
    );
    assert_eq!(
        float_ctx(
            &FloatProvider::Round(ProviderRef::inline(FloatProvider::Constant(-2.5))),
            &mut rng
        ),
        -2.0
    );
    // 0^0 is NaN and the safe wrapper turns it into 0.
    assert_eq!(
        float_ctx(
            &FloatProvider::Pow {
                base: ProviderRef::inline(FloatProvider::Constant(0.0)),
                exponent: ProviderRef::inline(FloatProvider::Constant(0.0)),
            },
            &mut rng
        ),
        0.0
    );
    // A zero divisor yields NaN, which the safe wrapper also reports as 0.
    assert_eq!(
        float_ctx(
            &FloatProvider::Mod(
                ProviderRef::inline(FloatProvider::Constant(5.5)),
                ProviderRef::inline(FloatProvider::Constant(0.0))
            ),
            &mut rng
        ),
        0.0
    );
    // Truncate follows the sign of the input.
    assert_eq!(
        float_ctx(
            &FloatProvider::Truncate(ProviderRef::inline(FloatProvider::Constant(-1.5))),
            &mut rng
        ),
        -1.0
    );
    // Average of an empty list divides by zero.
    assert_eq!(
        float_ctx(&FloatProvider::Average(Vec::new()), &mut rng),
        0.0
    );
}

#[test]
fn a_nan_operand_reaches_the_safe_wrapper() {
    let mut rng = seed();
    let nan = || {
        ProviderRef::inline(FloatProvider::Div(
            ProviderRef::inline(FloatProvider::Constant(0.0)),
            ProviderRef::inline(FloatProvider::Constant(0.0)),
        ))
    };
    let one = || ProviderRef::inline(FloatProvider::Constant(1.0));

    // `Math.max`/`Math.min` keep NaN, which `getFloat` then turns into 0.
    assert_eq!(
        float_ctx(&FloatProvider::Max(vec![nan(), one()]), &mut rng),
        0.0
    );
    assert_eq!(
        float_ctx(&FloatProvider::Min(vec![one(), nan()]), &mut rng),
        0.0
    );
    // `Math.pow(1, NaN)` is NaN, while C's `pow` returns 1.
    assert_eq!(
        float_ctx(
            &FloatProvider::Pow {
                base: one(),
                exponent: nan(),
            },
            &mut rng
        ),
        0.0
    );
}

#[test]
fn weighted_list_draws_one_bounded_random() {
    let provider = IntProvider::WeightedList(WeightedList::new(vec![
        WeightedValue::new(ProviderRef::inline(IntProvider::Constant(1)), 30),
        WeightedValue::new(ProviderRef::inline(IntProvider::Constant(0)), 70),
    ]));

    // The same seed must produce the same sequence, and the draw must consume
    // exactly one bounded random.
    let mut first = seed();
    let values: Vec<i32> = (0..8).map(|_| int_ctx(&provider, &mut first)).collect();
    let mut second = seed();
    let repeated: Vec<i32> = (0..8).map(|_| int_ctx(&provider, &mut second)).collect();
    assert_eq!(values, repeated);
    assert!(values.iter().all(|value| *value == 0 || *value == 1));

    let mut probe = seed();
    let _ = int_ctx(&provider, &mut probe);
    let mut reference = seed();
    let _ = reference.next_i32_bounded(100);
    assert_eq!(probe.next_i32(), reference.next_i32());
}

#[test]
fn uniform_matches_mth_next_int_and_next_float() {
    let mut rng = seed();
    let mut reference = seed();
    assert_eq!(
        int_ctx(
            &IntProvider::Uniform {
                min: ProviderRef::inline(IntProvider::Constant(3)),
                max: ProviderRef::inline(IntProvider::Constant(9)),
            },
            &mut rng
        ),
        reference.next_i32_bounded(7) + 3
    );
    assert_eq!(
        float_ctx(
            &FloatProvider::Uniform {
                min: ProviderRef::inline(FloatProvider::Constant(1.0)),
                max: ProviderRef::inline(FloatProvider::Constant(4.0)),
            },
            &mut rng
        ),
        reference.next_f32() * 3.0 + 1.0
    );
}

#[test]
fn parser_rejects_unknown_and_missing_types() {
    use simdnbt::owned::NbtCompound;

    let mut unknown = NbtCompound::new();
    unknown.insert("type", "minecraft:environment_attribute");
    assert!(matches!(
        parse_int_provider(&unknown),
        Err(ProviderParseError::UnsupportedType(_))
    ));

    let empty = NbtCompound::new();
    assert!(matches!(
        parse_float_provider(&empty),
        Err(ProviderParseError::MissingType)
    ));
}

#[test]
fn parser_builds_inline_arithmetic() {
    use simdnbt::owned::NbtCompound;

    let mut payload = NbtCompound::new();
    payload.insert("type", "minecraft:div");
    payload.insert("left", 7.0f32);
    payload.insert("right", 2.0f32);
    let provider = parse_float_provider(&payload).expect("inline div parses");
    let mut rng = seed();
    assert_eq!(float_ctx(&provider, &mut rng), 3.5);
}

#[test]
fn configured_cooking_providers_evaluate_against_the_block_state() {
    use crate::init_vanilla_registry;
    use crate::loot_table::loot_context_params;
    use crate::vanilla_blocks;

    init_vanilla_registry();
    let speed = crate::REGISTRY
        .context_float_providers
        .by_key(&Identifier::vanilla_static("cooking/speed_default"))
        .expect("configured cooking speed is registered");
    let bamboo = crate::REGISTRY
        .context_int_providers
        .by_key(&Identifier::vanilla_static("cooking/time_bamboo"))
        .expect("configured bamboo cooking time is registered");

    // A smoker is a fast-cooking block, a furnace is not.
    let mut rng = seed();
    let mut fast = LootContext::new(&mut rng).with_parameter(
        loot_context_params::BLOCK_STATE,
        vanilla_blocks::SMOKER.default_state(),
    );
    assert_eq!(speed.get_float(&mut fast), 2.0);
    assert_eq!(bamboo.get_int(&mut fast), 25);

    let mut rng = seed();
    let mut normal = LootContext::new(&mut rng).with_parameter(
        loot_context_params::BLOCK_STATE,
        vanilla_blocks::FURNACE.default_state(),
    );
    assert_eq!(speed.get_float(&mut normal), 1.0);
    assert_eq!(bamboo.get_int(&mut normal), 50);

    // Without a block state the predicate is false, which is the normal branch.
    let mut rng = seed();
    let mut empty = LootContext::new(&mut rng);
    assert_eq!(speed.get_float(&mut empty), 1.0);
}

#[test]
fn configured_registry_matches_the_extracted_catalog_size() {
    use crate::init_vanilla_registry;

    init_vanilla_registry();
    assert_eq!(
        crate::REGISTRY.context_float_providers.len(),
        crate::vanilla_context_number_providers::FLOAT_PROVIDER_COUNT
    );
    assert_eq!(
        crate::REGISTRY.context_int_providers.len(),
        crate::vanilla_context_number_providers::INT_PROVIDER_COUNT
    );
    assert_eq!(
        crate::REGISTRY.context_float_providers.len(),
        4,
        "vanilla registers four configured float providers"
    );
    assert_eq!(
        crate::REGISTRY.context_int_providers.len(),
        26,
        "vanilla registers twenty-six configured int providers"
    );
}

#[test]
fn every_configured_provider_evaluates() {
    use crate::init_vanilla_registry;
    use crate::loot_table::loot_context_params;
    use crate::vanilla_blocks;

    init_vanilla_registry();

    // A smoker exercises the fast-cooking branch of every configured value.
    let mut rng = seed();
    let mut ctx = LootContext::new(&mut rng).with_parameter(
        loot_context_params::BLOCK_STATE,
        vanilla_blocks::SMOKER.default_state(),
    );

    let mut evaluated = 0;
    for (key, provider) in crate::REGISTRY.context_float_providers.iter() {
        let value = provider.get_float(&mut ctx);
        assert!(value.is_finite(), "{key} produced {value}");
        evaluated += 1;
    }
    for (key, provider) in crate::REGISTRY.context_int_providers.iter() {
        // The compostable chances legitimately roll zero; this only proves the
        // generated payload evaluates without an error.
        let value = provider.get_int(&mut ctx);
        assert!(value >= 0, "{key} produced {value}");
        evaluated += 1;
    }
    assert_eq!(evaluated, 30, "4 float and 26 int configured values");
}
