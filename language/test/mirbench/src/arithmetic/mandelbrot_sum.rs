use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const MANDELBROT_POINTS: i64 = MANDELBROT_SIDE * MANDELBROT_SIDE;
const MANDELBROT_SIDE: i64 = 64;

declare_program! {
    /// Mandelbrot escape time sum over a fixed grid.
    pub const MANDELBROT_SUM,
    name: "mandelbrot_sum",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/mandelbrot_sum.mir")),
    entry: "mandelbrot_sum",
    expected: || {
        mandelbrot_sum(
            10_000,
            -2.0,
            -1.5,
            0.047619047619047616,
            4.0,
            2.0,
            32,
        )
    },
    default_args: |_interp| {
        vec![
            Value::int64(10_000),
            Value::float64(-2.0),
            Value::float64(-1.5),
            Value::float64(0.047619047619047616),
            Value::float64(4.0),
            Value::float64(2.0),
        ]
    },
    tags: &["arithmetic", "mandelbrot"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the mandelbrot benchmark.
fn mandelbrot_sum(
    iterations: i64,
    min_x: f64,
    min_y: f64,
    scale: f64,
    radius2: f64,
    two: f64,
    max_iter: i64,
) -> Value {
    // compute repeat count
    let repeats = clamp_min(iterations / MANDELBROT_POINTS, 1);
    let max_iter = clamp_min(max_iter, 1);

    // run escape time loops
    let mut total = 0i64;
    for _ in 0..repeats {
        for y in 0..MANDELBROT_SIDE {
            for x in 0..MANDELBROT_SIDE {
                // map grid point to complex plane
                let cx = min_x + (x as f64) * scale;
                let cy = min_y + (y as f64) * scale;
                let mut zx = 0.0;
                let mut zy = 0.0;
                let mut iter = 0i64;

                // iterate escape time
                while iter < max_iter {
                    let zx2 = zx * zx;
                    let zy2 = zy * zy;
                    if zx2 + zy2 > radius2 {
                        break;
                    }

                    let zxy = zx * zy;
                    zy = zxy * two + cy;
                    zx = zx2 - zy2 + cx;
                    iter += 1;
                }

                // accumulate iteration count
                total += iter;
            }
        }
    }

    // return value
    Value::int64(total)
}
