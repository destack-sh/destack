use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const HEADER_MAP_CAPACITY: i64 = 32;
const HEADER_MAP_SIZE: i64 = 16;

const HEADER_KEY_MUL: i64 = 3;
const HEADER_KEY_ADD: i64 = 1;
const HEADER_VAL_MUL: i64 = 5;
const HEADER_VAL_ADD: i64 = 2;

const MERGE_KEY_MUL: i64 = 7;
const MERGE_KEY_ADD: i64 = 3;
const MERGE_VAL_MUL: i64 = 11;
const MERGE_VAL_ADD: i64 = 5;

declare_program! {
    /// Merge two header maps using linear scans.
    pub const HEADER_MAP_MERGE,
    name: "header_map_merge",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/header_map_merge.mir")),
    entry: "header_map_merge",
    expected: || header_map_merge(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["memory", "map", "merge"],
    scales: &[
        scale_axis("ops", 0, 60000, 200000, 800000, true),
    ],
}

/// Compute the expected value for the header map merge benchmark.
fn header_map_merge(ops: i64) -> Value {
    // init primary map
    let mut keys = vec![0i64; HEADER_MAP_CAPACITY as usize];
    let mut values = vec![0i64; HEADER_MAP_CAPACITY as usize];
    for index in 0..HEADER_MAP_SIZE {
        keys[index as usize] = index
            .wrapping_mul(HEADER_KEY_MUL)
            .wrapping_add(HEADER_KEY_ADD);
        values[index as usize] = index
            .wrapping_mul(HEADER_VAL_MUL)
            .wrapping_add(HEADER_VAL_ADD);
    }

    // init secondary map
    let mut merge_keys = vec![0i64; HEADER_MAP_CAPACITY as usize];
    let mut merge_values = vec![0i64; HEADER_MAP_CAPACITY as usize];
    for index in 0..HEADER_MAP_CAPACITY {
        merge_keys[index as usize] = index
            .wrapping_mul(MERGE_KEY_MUL)
            .wrapping_add(MERGE_KEY_ADD);
        merge_values[index as usize] = index
            .wrapping_mul(MERGE_VAL_MUL)
            .wrapping_add(MERGE_VAL_ADD);
    }

    // merge maps
    let mut size = HEADER_MAP_SIZE;
    let mut acc = 0i64;
    let mut iter = 0i64;
    while iter < ops {
        let slot = iter % HEADER_MAP_CAPACITY;
        let key = merge_keys[slot as usize];
        let value = merge_values[slot as usize];

        let mut found = false;
        let mut scan = 0i64;
        while scan < size {
            if keys[scan as usize] == key {
                acc = acc.wrapping_add(values[scan as usize]);
                values[scan as usize] = value;
                found = true;
                break;
            }

            scan = scan.wrapping_add(1);
        }

        if !found {
            if size < HEADER_MAP_CAPACITY {
                keys[size as usize] = key;
                values[size as usize] = value;
                acc = acc.wrapping_add(value);
                size = size.wrapping_add(1);
            } else {
                acc = acc.wrapping_add(key);
            }
        }

        iter = iter.wrapping_add(1);
    }

    // return result
    acc = acc.wrapping_add(size).wrapping_add(iter);
    let mixed = mix_result(acc, ops);
    Value::int64(mixed)
}
