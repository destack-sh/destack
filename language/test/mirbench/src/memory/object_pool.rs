use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const POOL_CAPACITY: i64 = 32;
const POOL_HASH_MUL: i64 = 1103515245;
const POOL_HASH_ADD: i64 = 12345;

const POOL_TOKEN_ALLOC: i64 = 0;
const POOL_TOKEN_FREE: i64 = 1;

declare_program! {
    /// Exercise a simple object pool with alloc and free operations.
    pub const OBJECT_POOL,
    name: "object_pool",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/object_pool.mir")),
    entry: "object_pool",
    expected: || object_pool(50_000),
    default_args: |_interp| vec![Value::int64(50_000)],
    tags: &["memory", "pool", "managed"],
    scales: &[
        scale_axis("ops", 0, 50000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the object pool benchmark.
fn object_pool(ops: i64) -> Value {
    // init free list
    let mut next = vec![0i64; POOL_CAPACITY as usize];
    let mut values = vec![0i64; POOL_CAPACITY as usize];
    for index in 0..POOL_CAPACITY {
        next[index as usize] = index.wrapping_add(1);
        values[index as usize] = index;
    }
    next[(POOL_CAPACITY - 1) as usize] = -1;

    // run pool loop
    let mut head = 0i64;
    let mut acc = 0i64;
    let mut index = 0i64;
    while index < ops {
        let hash = index
            .wrapping_mul(POOL_HASH_MUL)
            .wrapping_add(POOL_HASH_ADD);
        let token = hash & 3;

        match token {
            POOL_TOKEN_ALLOC => {
                if head != -1 {
                    let slot = head;
                    let next_head = next[slot as usize];
                    let value = index.wrapping_add(7);
                    values[slot as usize] = value;
                    acc = acc.wrapping_add(value);
                    head = next_head;
                }
            }
            POOL_TOKEN_FREE => {
                let slot = index % POOL_CAPACITY;
                next[slot as usize] = head;
                head = slot;
                acc = acc.wrapping_add(slot);
            }
            _ => {
                let slot = index % POOL_CAPACITY;
                acc = acc.wrapping_add(values[slot as usize]);
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc.wrapping_add(head).wrapping_add(index);
    let mixed = mix_result(acc, ops);
    Value::int64(mixed)
}
