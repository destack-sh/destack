use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::common::{
    REFERENCE_BYTES, WORKLOAD_MUTATIONS, WORKLOAD_OBJECTS, allocate_local_object_graph,
    allocate_local_reference_array, allocate_shared_object_graph, allocate_shared_reference_array,
    local_heap, local_object_graph, shared_fixture,
};

/// Benchmark JS and TS shaped heap allocation workloads.
pub(crate) fn bench_heap_workload(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_workload");
    group.sample_size(50);
    group.throughput(Throughput::Elements(WORKLOAD_OBJECTS as u64));

    group.bench_function("local_object_graph", |bencher| {
        bencher.iter_batched(
            local_heap,
            |mut heap| black_box(allocate_local_object_graph(&mut heap)),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_object_graph", |bencher| {
        bencher.iter_batched(
            shared_fixture,
            |mut fixture| {
                black_box(allocate_shared_object_graph(
                    &fixture.heap,
                    &mut fixture.allocator,
                ))
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_reference_array", |bencher| {
        bencher.iter_batched(
            local_heap,
            |mut heap| black_box(allocate_local_reference_array(&mut heap)),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_reference_array", |bencher| {
        bencher.iter_batched(
            shared_fixture,
            |mut fixture| {
                black_box(allocate_shared_reference_array(
                    &fixture.heap,
                    &mut fixture.allocator,
                ))
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_fork_mutate_records", |bencher| {
        bencher.iter_batched(
            local_object_graph,
            |(mut heap, records)| {
                let mut fork = heap.fork().expect("heap fork should succeed");

                for (index, reference) in records.iter().take(WORKLOAD_MUTATIONS).enumerate() {
                    let value = index.wrapping_mul(17).to_le_bytes();
                    fork.write_heap_bytes(*reference, REFERENCE_BYTES, &value)
                        .expect("forked record write should succeed");
                }

                black_box(fork)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}
