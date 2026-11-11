use vecta_utils::random::RandomAccess;

pub(crate) fn get_blocks_to_grow_when_bonemealed(rng: &mut dyn RandomAccess) -> i32 {
    let mut grow_probability = 1.0;

    let mut count = 0;

    while rng.next_f64() < grow_probability {
        grow_probability *= 0.826;
        count += 1;
    }
    count
}
