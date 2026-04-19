use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Load and store loop on heap cells.
    pub const LOAD_STORE,
    name: "load_store",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/load_store.mir")),
    entry: "load_store",
    expected: || load_store(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the load store benchmark.
fn load_store(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut first = 0i64;
    let mut second = 1i64;

    // run fibonacci style loop
    while index < iterations {
        // update cell values
        let next = first.wrapping_add(second);
        first = second;
        second = next;

        // advance loop counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(second)
}
