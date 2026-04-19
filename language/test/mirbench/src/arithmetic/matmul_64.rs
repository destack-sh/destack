use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Matrix multiply with fixed 64x64 tiles.
    pub const MATMUL_64,
    name: "matmul_64",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/matmul_64.mir")),
    entry: "matmul_64",
    expected: || matmul_64(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "matrix"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the matmul sum for a tile size.
fn matmul_tile(iterations: i64, tile: i64) -> i64 {
    // normalize tile size
    let tile = clamp_min(tile, 1);
    let len = tile.wrapping_mul(tile);

    // init arrays
    let mut left = vec![0i64; len as usize];
    let mut right = vec![0i64; len as usize];
    let mut output = vec![0i64; len as usize];

    // fill input arrays
    for index in 0..len {
        let left_value = index.wrapping_mul(3).wrapping_add(1);
        let right_value = index.wrapping_mul(5).wrapping_add(7);
        left[index as usize] = left_value;
        right[index as usize] = right_value;
    }

    // compute iteration count
    let iterations = clamp_min(iterations / len, 1);

    // run matmul loop
    let mut total = 0i64;
    for _ in 0..iterations {
        // iterate output elements
        for idx in 0..len {
            let row = idx / tile;
            let col = idx % tile;
            let row_base = row * tile;
            let mut sum = 0i64;

            // accumulate inner product
            for k in 0..tile {
                let left_idx = row_base + k;
                let right_idx = k * tile + col;
                let product = left[left_idx as usize].wrapping_mul(right[right_idx as usize]);
                sum = sum.wrapping_add(product);
            }

            // store element and accumulate total
            output[idx as usize] = sum;
            total = total.wrapping_add(sum);
        }
    }

    // return sum
    total
}

/// Compute the expected value for the matmul 64 benchmark.
fn matmul_64(iterations: i64) -> Value {
    // return value
    Value::int64(matmul_tile(iterations, 64))
}
