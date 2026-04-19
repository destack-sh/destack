use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Element get/set on a managed array with sequential access.
    pub const ARRAY_WALK,
    name: "array_walk",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/array_walk.mir")),
    entry: "array_walk",
    expected: || Value::int64(499_500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "array"],
    scales: &[
        scale_axis("len", 0, 1000, 10000, 100000, true),
    ],
}
