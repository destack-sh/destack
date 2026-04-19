use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Non-tail recursive tree sum.
    pub const RECURSIVE_TREE_SUM,
    name: "recursive_tree_sum",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/recursive_tree_sum.mir")),
    entry: "recursive_tree_sum",
    expected: || recursive_tree_sum(10),
    default_args: |_interp| vec![Value::int64(10)],
    tags: &["calls", "recursion", "tree"],
    scales: &[
        scale_axis("depth", 0, 8, 10, 12, false),
    ],
}

/// Compute the expected value for the recursive tree sum benchmark.
fn recursive_tree_sum(depth: i64) -> Value {
    // return recursive sum
    let mixed = mix_result(tree_sum(depth), depth);
    Value::int64(mixed)
}
fn tree_sum(depth: i64) -> i64 {
    // base case
    if depth == 0 {
        return 1;
    }

    // recursive case
    let next = depth.wrapping_sub(1);
    let left = tree_sum(next);
    let right = tree_sum(next);
    left.wrapping_add(right).wrapping_add(1)
}
