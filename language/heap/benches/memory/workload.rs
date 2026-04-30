use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};
use destack_heap::{Heap, HeapReference};

use crate::common::{
    REFERENCE_BYTES, WORKLOAD_MUTATIONS, WORKLOAD_OBJECTS, allocate_local_object_graph,
    allocate_local_reference_array, allocate_shared_object_graph, allocate_shared_reference_array,
    local_heap, local_object_graph, shared_fixture,
};

/// Store one scalar field in a forked local record.
#[inline(always)]
fn store_local_record_value(heap: &mut Heap, reference: HeapReference, value: usize) {
    heap.write_barrier(reference, REFERENCE_BYTES, REFERENCE_BYTES)
        .expect("heap barrier should record");
    let address = heap.heap_base_address() + reference.offset() + REFERENCE_BYTES;

    // store after the fork barrier has detached the page
    unsafe {
        (address as *mut usize).write(value);
    }
}

/// Benchmark JS and TS shaped heap allocation workloads.
pub(crate) fn bench_heap_workload(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_workload");
    group.sample_size(50);
    group.throughput(Throughput::Elements(WORKLOAD_OBJECTS as u64));

    // local records point at local leaf records
    group.bench_function("local_object_graph", |bencher| {
        bencher.iter_batched(
            local_heap,
            |mut heap| black_box(allocate_local_object_graph(&mut heap)),
            BatchSize::SmallInput,
        );
    });

    // shared records point at shared leaf records
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

    // local repeated reference maps model fixed pointer arrays
    group.bench_function("local_reference_array", |bencher| {
        bencher.iter_batched(
            local_heap,
            |mut heap| black_box(allocate_local_reference_array(&mut heap)),
            BatchSize::SmallInput,
        );
    });

    // shared repeated reference maps model fixed pointer arrays
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

    // mutate scalar fields after forking an object graph
    group.bench_function("local_fork_mutate_records", |bencher| {
        bencher.iter_batched(
            local_object_graph,
            |(mut heap, records)| {
                let mut fork = heap.fork().expect("heap fork should succeed");

                for (index, reference) in records.iter().take(WORKLOAD_MUTATIONS).enumerate() {
                    let value = index.wrapping_mul(17);
                    store_local_record_value(&mut fork, *reference, value);
                }

                black_box(fork)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}
