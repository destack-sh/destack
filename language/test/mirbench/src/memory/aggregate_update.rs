use super::super::{Program, scale_axis};
use destack_vm::Value;

/// Default iteration count for aggregate updates.
const DEFAULT_ITERS: i64 = 1000;

/// Compute the expected aggregate update output.
fn expected_aggregate_update(iterations: i64) -> i64 {
    // seed aggregate values
    let mut struct_middle = 2i64;
    let mut tuple_head = 1i64;
    let mut array = [1i64, 2i64, 3i64, 2i64];

    // walk the iteration count
    let mut index = 0i64;
    while index < iterations {
        let is_odd = (index & 1) == 1;
        let plus = struct_middle.wrapping_add(5);
        let minus = struct_middle.wrapping_sub(5);
        let updated = if is_odd { plus } else { minus };
        struct_middle = updated;
        let slot = (index & 3) as usize;
        let next = array[slot].wrapping_add(updated);
        array[slot] = next;
        tuple_head = tuple_head.wrapping_add(updated);
        index += 1;
    }

    // return the combined aggregates
    let combined = struct_middle.wrapping_add(tuple_head);
    combined.wrapping_add(array[2])
}

declare_program! {
    /// Value aggregate updates with field.set, element.set, and select.
    pub const AGGREGATE_UPDATE,
    name: "aggregate_update",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/aggregate_update.mir")),
    entry: "aggregate_update",
    expected: || Value::int64(expected_aggregate_update(DEFAULT_ITERS)),
    default_args: |_interp| vec![Value::int64(DEFAULT_ITERS)],
    tags: &["memory", "aggregate", "select"],
    scales: &[
        scale_axis("iter", 0, DEFAULT_ITERS, 10000, 100000, true),
    ],
}
