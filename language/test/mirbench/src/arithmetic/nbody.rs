use super::super::common::{abs_i64, clamp_min};
use super::super::{Program, scale_axis};
use destack_vm::Value;

const NBODY_MIN_COUNT: i64 = 4;
const NBODY_STEP_SCALE: i64 = 64;

declare_program! {
    /// Integer based n body integration loop.
    pub const NBODY,
    name: "nbody",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/nbody.mir")),
    entry: "nbody",
    expected: || nbody(10_000, 32),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "nbody"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the n body benchmark.
fn nbody(iterations: i64, body_count: i64) -> Value {
    // normalize body count
    let body_count = clamp_min(body_count, NBODY_MIN_COUNT);

    // init arrays
    let mut positions = vec![0i64; body_count as usize];
    let mut velocities = vec![0i64; body_count as usize];

    // fill body state
    for index in 0..body_count {
        let position = index.wrapping_add(1);
        let velocity = index.wrapping_mul(2).wrapping_add(1);
        positions[index as usize] = position;
        velocities[index as usize] = velocity;
    }

    // compute step count
    let steps = clamp_min(iterations / NBODY_STEP_SCALE, 1);

    // run integration loop
    for _ in 0..steps {
        // update each body
        for i in 0..body_count {
            let pos_i = positions[i as usize];
            let mut acc = 0i64;

            // accumulate pairwise forces
            for j in 0..body_count {
                if i == j {
                    continue;
                }

                let diff = positions[j as usize].wrapping_sub(pos_i);
                let denom = abs_i64(diff).wrapping_add(1);
                let force = diff.wrapping_div(denom);
                acc = acc.wrapping_add(force);
            }

            // integrate velocity and position
            let vel = velocities[i as usize].wrapping_add(acc);
            velocities[i as usize] = vel;
            positions[i as usize] = pos_i.wrapping_add(vel);
        }
    }

    // sum final positions
    let mut total = 0i64;
    for value in positions {
        total = total.wrapping_add(value);
    }

    // return value
    Value::int64(total)
}
