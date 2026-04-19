use super::super::{Program, scale_axis_range};
use destack_vm::Value;

declare_program! {
    /// Walk a linked list with proper node structure.
    ///
    /// Each node has two slots: [value, next_pointer].
    /// Builds list n -> (n-1) -> ... -> 1 -> sentinel(0), then traverses it.
    pub const LINKED_WALK,
    name: "linked_walk",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/linked_walk.mir")),
    entry: "linked_walk",
    // sum of 1..100 = 100*101/2 = 5050
    expected: || Value::int64(5050),
    default_args: |_interp| vec![Value::int64(100)],
    tags: &["memory", "list"],
    scales: &[
        scale_axis_range("nodes", 0, 100, 1000, 2000, true, 1, 2000),
    ],
}
