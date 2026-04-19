use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Crc style mixing with rotate and byteswap intrinsics.
    pub const CRC32_LIKE,
    name: "crc32_like",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/crc32_like.mir")),
    entry: "crc32_like",
    expected: || crc32_like(10_000, 1311768467463790320),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(1311768467463790320)],
    tags: &["intrinsics", "crc"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the crc32 like benchmark.
fn crc32_like(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut crc = seed;

    // run crc loop
    while index < iterations {
        // mix next byte
        let mixed = index.wrapping_mul(31).wrapping_add(17);
        let byte = mixed & 255;
        let xored = crc ^ byte;
        let rotated = (xored as u64).rotate_left(5) as i64;
        let swapped = rotated.swap_bytes();
        let mulled = swapped.wrapping_mul(517762881);
        let xored_next = mulled ^ byte;
        let rotated_next = (xored_next as u64).rotate_right(11) as i64;
        let mixed_next = rotated_next.wrapping_mul(2246822519);

        // advance state
        crc = mixed_next;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(crc)
}
