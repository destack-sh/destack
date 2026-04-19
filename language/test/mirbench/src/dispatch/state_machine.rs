use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Simple state machine cycling through three states.
    pub const STATE_MACHINE,
    name: "state_machine",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/state_machine.mir")),
    entry: "state_machine",
    expected: || Value::int64(19999),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "state"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
