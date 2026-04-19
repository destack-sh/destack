use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const SMALL_MAP_CAPACITY: i64 = 64;
const SMALL_MAP_HASH_MUL: i64 = 1103515245;
const SMALL_MAP_HASH_ADD: i64 = 12345;

declare_program! {
    /// Small map with linear scan and insert.
    pub const SMALL_MAP,
    name: "small_map",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/small_map.mir")),
    entry: "small_map",
    expected: || small_map(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["memory", "map", "linear"],
    scales: &[
        scale_axis("ops", 0, 60000, 200000, 800000, true),
    ],
}

/// Compute the expected value for the small map benchmark.
fn small_map(ops: i64) -> Value {
    // init table
    let mut keys = vec![0i64; SMALL_MAP_CAPACITY as usize];
    let mut values = vec![0i64; SMALL_MAP_CAPACITY as usize];
    let mut size = 0i64;
    let mut acc = 0i64;

    // run map loop
    let mut index = 0i64;
    while index < ops {
        let hash = index
            .wrapping_mul(SMALL_MAP_HASH_MUL)
            .wrapping_add(SMALL_MAP_HASH_ADD);
        let key = hash ^ (hash >> 16);
        let value = hash.wrapping_add(1);

        let mut slot = 0i64;
        let mut found = false;
        while slot < size {
            if keys[slot as usize] == key {
                acc = acc.wrapping_add(values[slot as usize]);
                values[slot as usize] = value;
                found = true;
                break;
            }

            slot = slot.wrapping_add(1);
        }

        if !found {
            if size < SMALL_MAP_CAPACITY {
                keys[size as usize] = key;
                values[size as usize] = value;
                size = size.wrapping_add(1);
                acc = acc.wrapping_add(value);
            } else {
                acc = acc.wrapping_add(key);
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc.wrapping_add(size);
    let mixed = mix_result(acc, ops);
    Value::int64(mixed)
}
