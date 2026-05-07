use criterion::Criterion;

use crate::benchmark;
use crate::program::Program;

/// Benchmark VM dispatch over memory operations.
pub(crate) fn bench_memory(criterion: &mut Criterion) {
    let mut group = benchmark::group(criterion, "vm_dispatch_memory");

    benchmark::program(
        &mut group,
        Program {
            name: "heap_load_store",
            entry: "heapLoadStore",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "heap_allocate",
            entry: "heapAllocate",
        },
    );

    group.finish();
}
