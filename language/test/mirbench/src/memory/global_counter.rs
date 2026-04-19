use super::super::{Program, scale_axis};
use destack_vm::Value;

/// Default iteration count for the global counter.
const DEFAULT_ITERS: i64 = 1000;
/// Seed used for global updates.
const GLOBAL_SEED: i64 = 6364136223846793005;

/// Compute the expected global counter output.
fn expected_global_counter(iterations: i64) -> i64 {
    // seed the global state
    let mut counter = 0i64;

    // walk the iteration count
    let mut index = 0i64;
    while index < iterations {
        let next = counter.wrapping_add(GLOBAL_SEED);
        counter = next ^ index;
        index += 1;
    }

    // return the final counter
    counter
}

declare_program! {
    /// Loop that mixes global.const and global.address access.
    pub const GLOBAL_COUNTER,
    name: "global_counter",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/global_counter.mir")),
    entry: "global_counter",
    expected: || Value::int64(expected_global_counter(DEFAULT_ITERS)),
    default_args: |_interp| vec![Value::int64(DEFAULT_ITERS)],
    tags: &["memory", "global"],
    scales: &[
        scale_axis("iter", 0, DEFAULT_ITERS, 10000, 100000, true),
    ],
}
