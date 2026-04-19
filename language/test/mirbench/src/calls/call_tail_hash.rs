use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Tail recursive hash fold using multiply and xor mixing.
    pub const CALL_TAIL_HASH,
    name: "call_tail_hash",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_tail_hash.mir")),
    entry: "call_tail_hash",
    expected: || call_tail_hash(100_000, 5381),
    default_args: |_interp| vec![Value::int64(100_000)],
    tags: &["calls", "call", "tail", "hash"],
    scales: &[
        scale_axis("iter", 0, 100000, 1000000, 10000000, true),
    ],
}

/// Compute the expected value for the tail hash benchmark.
fn call_tail_hash(iterations: i64, seed: i64) -> Value {
    // init state
    let mut remaining = iterations;
    let mut hash = seed;

    // run hash fold
    while remaining != 0 {
        // update hash state
        let next = remaining.wrapping_sub(1);
        let mixed = hash.wrapping_mul(33);
        let shifted = hash >> 1;
        let xored = next ^ shifted;
        hash = mixed.wrapping_add(xored);

        // advance loop counter
        remaining = next;
    }

    // return value
    Value::int64(hash)
}
