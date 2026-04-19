use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const LRU_CAPACITY: i64 = 16;
const LRU_HASH_MUL: i64 = 1103515245;
const LRU_HASH_ADD: i64 = 12345;

declare_program! {
    /// Exercise a small LRU cache with hits, misses, and evictions.
    pub const LRU_CACHE,
    name: "lru_cache",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/lru_cache.mir")),
    entry: "lru_cache",
    expected: || lru_cache(50_000),
    default_args: |_interp| vec![Value::int64(50_000)],
    tags: &["memory", "cache", "lru"],
    scales: &[
        scale_axis("ops", 0, 50000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the LRU cache benchmark.
fn lru_cache(ops: i64) -> Value {
    // init cache
    let mut keys = vec![0i64; LRU_CAPACITY as usize];
    let mut ages = vec![0i64; LRU_CAPACITY as usize];
    let mut size = 0i64;
    let mut acc = 0i64;
    let mut clock = 1i64;

    // run cache loop
    let mut index = 0i64;
    while index < ops {
        let hash = index.wrapping_mul(LRU_HASH_MUL).wrapping_add(LRU_HASH_ADD);
        let key = hash ^ (hash >> 16);

        let mut slot = 0i64;
        let mut hit = false;
        while slot < size {
            if keys[slot as usize] == key {
                acc = acc.wrapping_add(ages[slot as usize]);
                ages[slot as usize] = clock;
                hit = true;
                break;
            }

            slot = slot.wrapping_add(1);
        }

        if !hit {
            let target = if size < LRU_CAPACITY {
                let target = size;
                size = size.wrapping_add(1);
                target
            } else {
                let mut min_slot = 0i64;
                let mut min_age = ages[0];
                let mut scan = 1i64;

                while scan < LRU_CAPACITY {
                    let age = ages[scan as usize];
                    if age < min_age {
                        min_age = age;
                        min_slot = scan;
                    }

                    scan = scan.wrapping_add(1);
                }

                min_slot
            };

            keys[target as usize] = key;
            ages[target as usize] = clock;
            acc = acc.wrapping_add(key);
        }

        index = index.wrapping_add(1);
        clock = clock.wrapping_add(1);
    }

    // return result
    acc = acc.wrapping_add(size).wrapping_add(clock);
    let mixed = mix_result(acc, ops);
    Value::int64(mixed)
}
