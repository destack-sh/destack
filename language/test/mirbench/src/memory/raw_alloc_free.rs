use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Raw allocation with immediate free per iteration.
    pub const RAW_ALLOC_FREE,
    name: "raw_alloc_free",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/raw_alloc_free.mir")),
    entry: "raw_alloc_free",
    expected: || Value::int64(499_500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "alloc", "raw"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
