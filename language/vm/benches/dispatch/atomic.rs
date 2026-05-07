use criterion::Criterion;

use crate::benchmark;
use crate::program::Program;

/// Benchmark VM dispatch over atomic memory operations.
pub(crate) fn bench_atomic(criterion: &mut Criterion) {
    let mut group = benchmark::group(criterion, "vm_dispatch_atomic");

    benchmark::program(
        &mut group,
        Program {
            name: "atomic_managed_heap_add_loop",
            entry: "atomicManagedHeapAddLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "atomic_owned_heap_add_loop",
            entry: "atomicOwnedHeapAddLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "atomic_raw_add_loop",
            entry: "atomicRawAddLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "atomic_shared_heap_add_loop",
            entry: "atomicSharedHeapAddLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "atomic_shared_owned_heap_add_loop",
            entry: "atomicSharedOwnedHeapAddLoop",
        },
    );

    benchmark::program(
        &mut group,
        Program {
            name: "atomic_shared_raw_add_loop",
            entry: "atomicSharedRawAddLoop",
        },
    );

    group.finish();
}
