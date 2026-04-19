use super::super::common::{clamp_min, memcmp_bytes};
use super::super::{Program, scale_axis};
use destack_vm::Value;

const STRING_SCAN_MASK: i64 = 31;

declare_program! {
    /// String builder style copy and scan workload.
    pub const STRING_BUILDER_SCAN,
    name: "string_builder_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/string_builder_scan.mir")),
    entry: "string_builder_scan",
    expected: || string_builder_scan(10_000, 1024, 32, 7),
    default_args: |_interp| {
        vec![
            Value::int64(10_000),
            Value::int64(1024),
            Value::int64(32),
            Value::int64(7),
        ]
    },
    tags: &["intrinsics", "string"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
        scale_axis("len", 1, 1024, 4096, 16384, false),
    ],
}

/// Compute the expected value for the string builder scan benchmark.
fn string_builder_scan(iterations: i64, buffer_len: i64, chunk_len: i64, seed: i64) -> Value {
    // normalize buffer sizes
    let buffer_len = clamp_min(buffer_len, 1);
    let mut chunk_len = clamp_min(chunk_len, 1);
    if chunk_len > buffer_len {
        chunk_len = buffer_len;
    }

    // build pattern data
    let mut pattern = vec![0u8; chunk_len as usize];
    let mut index = 0i64;
    while index < chunk_len {
        let value = index.wrapping_mul(3).wrapping_add(seed);
        pattern[index as usize] = (value & 255) as u8;
        index = index.wrapping_add(1);
    }

    // initialize buffer
    let mut buffer = vec![0u8; buffer_len as usize];
    let sentinel = (seed & STRING_SCAN_MASK) as u8;
    let span = buffer_len.wrapping_sub(chunk_len).wrapping_add(1);
    let mut acc = 0i64;

    // run builder loop
    let mut iter = 0i64;
    while iter < iterations {
        // clear buffer periodically
        if (iter & 7) == 0 {
            buffer.fill(0);
        }

        // compute copy offset
        let offset = iter.wrapping_mul(chunk_len).wrapping_rem(span) as usize;

        // compare current contents
        let cmp = memcmp_bytes(&buffer[offset..offset + chunk_len as usize], &pattern);
        acc = acc.wrapping_add(cmp as i64);

        // copy pattern into buffer
        buffer[offset..offset + chunk_len as usize].copy_from_slice(&pattern);

        // scan for sentinel byte
        let mut count = 0i64;
        let mut scan = 0i64;
        while scan < chunk_len {
            let byte = buffer[offset + scan as usize];
            if byte == sentinel {
                count = count.wrapping_add(1);
            }
            scan = scan.wrapping_add(1);
        }

        // update accumulator
        acc = acc.wrapping_add(count);

        // advance counter
        iter = iter.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
