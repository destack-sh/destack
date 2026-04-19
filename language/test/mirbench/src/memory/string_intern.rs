use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const INTERN_TABLE_LEN: i64 = 256;
const INTERN_HASH_MUL: i64 = 1103515245;
const INTERN_HASH_ADD: i64 = 12345;
const INTERN_LEN_MASK: i64 = 15;

declare_program! {
    /// Intern pseudo string hashes into a linear table.
    pub const STRING_INTERN,
    name: "string_intern",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/string_intern.mir")),
    entry: "string_intern",
    expected: || string_intern(40_000),
    default_args: |_interp| vec![Value::int64(40_000)],
    tags: &["memory", "string", "intern"],
    scales: &[
        scale_axis("ops", 0, 40000, 150000, 500000, true),
    ],
}

/// Compute the expected value for the string intern benchmark.
fn string_intern(ops: i64) -> Value {
    // init table
    let mut hashes = vec![0i64; INTERN_TABLE_LEN as usize];
    let mut lens = vec![0i64; INTERN_TABLE_LEN as usize];
    let mut count = 0i64;
    let mut acc = 0i64;

    // run intern loop
    let mut index = 0i64;
    while index < ops {
        let hash = index
            .wrapping_mul(INTERN_HASH_MUL)
            .wrapping_add(INTERN_HASH_ADD);
        let len = (hash & INTERN_LEN_MASK).wrapping_add(1);

        // scan for existing entry
        let mut probe = 0i64;
        let mut found = false;
        while probe < count {
            let slot_hash = hashes[probe as usize];
            let slot_len = lens[probe as usize];
            if slot_hash == hash && slot_len == len {
                found = true;
                acc = acc.wrapping_add(1);
                break;
            }

            probe = probe.wrapping_add(1);
        }

        // insert when missing
        if !found {
            if count < INTERN_TABLE_LEN {
                hashes[count as usize] = hash;
                lens[count as usize] = len;
                count = count.wrapping_add(1);
                acc = acc.wrapping_add(2);
            } else {
                acc = acc.wrapping_add(3);
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc.wrapping_add(count);
    let mixed = mix_result(acc, ops);
    Value::int64(mixed)
}
