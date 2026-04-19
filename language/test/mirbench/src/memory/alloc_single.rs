use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Single allocation per iteration.
    pub const ALLOC_SINGLE,
    name: "alloc_single",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/alloc_single.mir")),
    entry: "alloc_single",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "alloc"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
