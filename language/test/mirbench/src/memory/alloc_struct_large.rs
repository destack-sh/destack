use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Allocate larger structs with multiple field stores per iteration.
    pub const ALLOC_STRUCT_LARGE,
    name: "alloc_struct_large",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/alloc_struct_large.mir")),
    entry: "alloc_struct_large",
    expected: || Value::int64(499_500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "alloc"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
