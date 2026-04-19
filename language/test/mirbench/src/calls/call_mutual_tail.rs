use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Mutually recursive tail calls with alternating increments.
    pub const CALL_MUTUAL_TAIL,
    name: "call_mutual_tail",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_mutual_tail.mir")),
    entry: "call_mutual_tail",
    expected: || call_mutual_tail(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["calls", "call", "tail", "mutual"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the mutual tail call benchmark.
fn call_mutual_tail(iterations: i64) -> Value {
    // init state
    let mut remaining = iterations;
    let mut acc_even = 0i64;
    let mut acc_odd = 0i64;
    let mut acc_mix = 0i64;
    let mut is_even = true;

    // run tail loop
    while remaining > 0 {
        // apply alternating increments
        if is_even {
            acc_even = acc_even.wrapping_add(2);
            acc_odd = acc_odd.wrapping_add(1);
            acc_mix ^= remaining;
        } else {
            let next_even = acc_even.wrapping_add(1);
            acc_mix = acc_mix.wrapping_add(acc_even);
            acc_even = next_even;
            acc_odd = acc_odd.wrapping_add(2);
        }

        // advance state
        remaining = remaining.wrapping_sub(1);
        is_even = !is_even;
    }

    // return value
    Value::int64(acc_even.wrapping_add(acc_odd).wrapping_add(acc_mix))
}
