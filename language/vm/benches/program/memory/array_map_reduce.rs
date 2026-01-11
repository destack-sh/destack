use super::super::{Program, scale_axis};
use destack_vm::memory::Value;

declare_program! {
    /// Map and reduce over a managed array.
    pub(crate) const ARRAY_MAP_REDUCE,
    name: "array_map_reduce",
    source: include_str!("array_map_reduce.mir"),
    entry: "array_map_reduce",
    expected: || Value::int64(1_505_500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "array"],
    scales: &[
        scale_axis("len", 0, 1000, 10000, 100000, true),
    ],
}
