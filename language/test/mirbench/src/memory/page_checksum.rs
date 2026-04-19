use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Fill a page and compute a rolling checksum.
    pub const PAGE_CHECKSUM,
    name: "page_checksum",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/page_checksum.mir")),
    entry: "page_checksum",
    expected: || page_checksum(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "checksum", "scan"],
    scales: &[
        scale_axis("len", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the page checksum benchmark.
fn page_checksum(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // compute checksum directly
    while index < length {
        let value = index.wrapping_mul(1103515245).wrapping_add(12345);
        let mixed = value.wrapping_mul(3);
        acc = acc.wrapping_add(mixed).wrapping_add(index);

        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
