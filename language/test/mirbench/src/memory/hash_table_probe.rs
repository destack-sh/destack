use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const HASH_TABLE_MIN_LEN: i64 = 8;

declare_program! {
    /// Hash table style linear probing with mixed operations.
    pub const HASH_TABLE_PROBE,
    name: "hash_table_probe",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/hash_table_probe.mir")),
    entry: "hash_table_probe",
    expected: || hash_table_probe(10_000, 1024, 7),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(1024), Value::int64(7)],
    tags: &["memory", "hash"],
    scales: &[
        scale_axis("ops", 0, 10000, 100000, 1000000, true),
        scale_axis("len", 1, 1024, 4096, 16384, false),
    ],
}

/// Compute the expected value for the hash table probe benchmark.
fn hash_table_probe(ops: i64, table_len: i64, seed: i64) -> Value {
    // normalize table size
    let table_len = clamp_min(table_len, HASH_TABLE_MIN_LEN);
    let len = table_len as usize;

    // initialize table storage
    let empty = -1i64;
    let deleted = -2i64;
    let mut keys = vec![empty; len];
    let mut values = vec![0i64; len];

    // run probe loop
    let mut index = 0i64;
    let mut acc = 0i64;
    while index < ops {
        // compute hash and key
        let hash = index.wrapping_mul(1103515245).wrapping_add(seed);
        let shifted = hash >> 16;
        let key = hash ^ shifted;
        let key_norm = key & 0x7fffffff;

        // derive start slot and action
        let start = key_norm.wrapping_rem(table_len) as usize;
        let action = hash & 3;

        // probe table according to action
        let mut probe = 0usize;
        let mut slot = start;
        let mut found = false;
        if action == 0 {
            // insert or update entry
            while probe < len {
                let existing = keys[slot];
                if existing == key {
                    values[slot] = hash;
                    acc = acc.wrapping_add(hash);
                    found = true;
                    break;
                }

                if existing == empty || existing == deleted {
                    keys[slot] = key;
                    values[slot] = hash;
                    acc = acc.wrapping_add(hash);
                    found = true;
                    break;
                }

                slot = (slot + 1) % len;
                probe += 1;
            }
        } else if action == 1 {
            // lookup entry
            while probe < len {
                let existing = keys[slot];
                if existing == key {
                    acc = acc.wrapping_add(values[slot]);
                    found = true;
                    break;
                }

                if existing == empty {
                    break;
                }

                slot = (slot + 1) % len;
                probe += 1;
            }
        } else {
            // delete entry
            while probe < len {
                let existing = keys[slot];
                if existing == key {
                    acc = acc.wrapping_add(values[slot]);
                    keys[slot] = deleted;
                    values[slot] = 0;
                    found = true;
                    break;
                }

                if existing == empty {
                    break;
                }

                slot = (slot + 1) % len;
                probe += 1;
            }
        }

        // apply miss penalty
        if !found {
            acc = acc.wrapping_add(key_norm);
        }

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
