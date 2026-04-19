use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Symbol table insertion and lookup with linear probing.
    pub const SYMBOL_TABLE,
    name: "symbol_table",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/symbol_table.mir")),
    entry: "symbol_table",
    expected: || symbol_table(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "hash", "table"],
    scales: &[
        scale_axis("len", 0, 10000, 100000, 1000000, true),
    ],
}

const TABLE_CAPACITY: usize = 128;

/// Compute the expected value for the symbol table benchmark.
fn symbol_table(length: i64) -> Value {
    // init tables
    let mut keys = vec![-1i64; TABLE_CAPACITY];
    let mut values = vec![0i64; TABLE_CAPACITY];
    let mut acc = 0i64;

    // process keys
    let mut index = 0i64;
    while index < length {
        let key = index.wrapping_mul(1103515245).wrapping_add(12345) & 255;
        let mut probe = 0usize;
        loop {
            if probe >= TABLE_CAPACITY {
                break;
            }
            let slot = keys[probe];
            if slot == key {
                acc = acc.wrapping_add(values[probe]);
                break;
            }
            if slot == -1 {
                keys[probe] = key;
                values[probe] = index;
                acc = acc.wrapping_add(index);
                break;
            }
            probe += 1;
        }

        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
