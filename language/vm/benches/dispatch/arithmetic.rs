use criterion::Criterion;

use crate::benchmark;
use crate::program::Program;

/// Benchmark VM dispatch over integer arithmetic.
pub(crate) fn bench_integer_arithmetic(criterion: &mut Criterion) {
    let mut group = benchmark::group(criterion, "vm_dispatch_arithmetic");

    benchmark::program(
        &mut group,
        Program {
            name: "integer_add_loop",
            entry: "integerAddLoop",
        },
    );

    group.finish();
}
