use criterion::Criterion;

use crate::benchmark;
use crate::program::Program;

/// Benchmark VM dispatch over packed vector operations.
pub(crate) fn bench_vector(criterion: &mut Criterion) {
    let mut group = benchmark::group(criterion, "vm_dispatch_vector");

    benchmark::program(
        &mut group,
        Program {
            name: "vector_i32x4_add_body_loop",
            entry: "vectorI32x4AddBodyLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "vector_i32x4_add_loop",
            entry: "vectorI32x4AddLoop",
        },
    );

    group.finish();
}
