use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Allocate small arrays sized to stay inline.
    pub const ALLOC_ARRAY_SMALL,
    name: "alloc_array_small",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/alloc_array_small.mir")),
    entry: "alloc_array_small",
    expected: || Value::int64(499_500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "alloc", "array"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
