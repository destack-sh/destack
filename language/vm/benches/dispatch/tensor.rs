use criterion::Criterion;

use crate::benchmark;
use crate::program::Program;

/// Benchmark VM dispatch over contiguous tensor operations.
pub(crate) fn bench_tensor(criterion: &mut Criterion) {
    let mut group = benchmark::group(criterion, "vm_dispatch_tensor");

    benchmark::program(
        &mut group,
        Program {
            name: "tensor_contiguous_add_body_loop",
            entry: "tensorContiguousAddBodyLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "tensor_contiguous_add_loop",
            entry: "tensorContiguousAddLoop",
        },
    );

    group.finish();
}
