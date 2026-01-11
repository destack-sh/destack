use super::super::{Program, scale_axis};
use destack_vm::memory::Value;

declare_program! {
    /// Sixteen way switch dispatch testing larger jump tables.
    pub(crate) const SWITCH_16WAY,
    name: "switch_16way",
    source: include_str!("switch_16way.mir"),
    entry: "switch_16way",
    expected: || Value::int64(75000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "switch"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
