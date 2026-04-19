use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Switch-heavy loop with mixed step sizes.
    pub const LOOP_SWITCH_MIX,
    name: "loop_switch_mix",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/loop_switch_mix.mir")),
    entry: "loop_switch_mix",
    expected: || loop_switch_mix(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "switch", "loop"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the loop switch mix benchmark.
fn loop_switch_mix(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // compute selector
        let mixed = index
            .wrapping_mul(1103515245)
            .wrapping_add(12345)
            .wrapping_rem(6);

        // apply per-case update
        let (next_acc, step) = match mixed {
            0 => (acc.wrapping_add(index), 1),
            1 => (acc.wrapping_add(index.wrapping_mul(2)), 2),
            2 => (acc.wrapping_add(index.wrapping_add(7)), 1),
            3 => (acc.wrapping_add(index ^ acc), 1),
            4 => (acc.wrapping_add(3).wrapping_add(index), 2),
            _ => (acc.wrapping_sub(index), 1),
        };

        // advance state
        acc = next_acc;
        index = index.wrapping_add(step);
    }

    // return result
    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
