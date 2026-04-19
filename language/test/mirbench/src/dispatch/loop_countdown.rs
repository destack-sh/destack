use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Simple countdown loop measuring baseline dispatch overhead.
    pub const LOOP_COUNTDOWN,
    name: "loop_countdown",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/loop_countdown.mir")),
    entry: "loop_countdown",
    expected: || Value::int64(0),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "loop"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
