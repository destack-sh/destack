use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Four way switch dispatch testing jump tables.
    pub const SWITCH_4WAY,
    name: "switch_4way",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/switch_4way.mir")),
    entry: "switch_4way",
    // 0+1+2+3 = 6 per 4 iters, so 10000/4 * 6 = 15000
    expected: || Value::int64(15000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "switch"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
