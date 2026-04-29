use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::common::{
    FIELD_ACCESS_PASSES, REFERENCE_BYTES, WORKLOAD_OBJECTS, allocate_shared_object_graph,
    local_object_graph, shared_fixture,
};

/// Benchmark record field access over realistic heap objects.
pub(crate) fn bench_heap_field_access(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_field_access");
    let total_fields = (WORKLOAD_OBJECTS * FIELD_ACCESS_PASSES) as u64;
    group.throughput(Throughput::Elements(total_fields));

    group.bench_function("local_record_field_write_checked", |bencher| {
        bencher.iter_batched(
            local_object_graph,
            |(mut heap, records)| {
                for pass in 0..FIELD_ACCESS_PASSES {
                    for (index, reference) in records.iter().enumerate() {
                        let value = (pass ^ index).to_le_bytes();
                        heap.write_heap_bytes(*reference, REFERENCE_BYTES, &value)
                            .expect("local field write should succeed");
                    }
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_record_field_write_resolved", |bencher| {
        bencher.iter_batched(
            local_object_graph,
            |(mut heap, records)| {
                for pass in 0..FIELD_ACCESS_PASSES {
                    for (index, reference) in records.iter().enumerate() {
                        let address = heap
                            .heap_address_mut(*reference, REFERENCE_BYTES, REFERENCE_BYTES)
                            .expect("local field address should resolve");

                        // field.addr plus raw store
                        unsafe {
                            std::ptr::write_unaligned(address.cast::<usize>(), pass ^ index);
                        }
                    }
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_record_field_write_direct", |bencher| {
        bencher.iter_batched(
            || {
                let (mut heap, records) = local_object_graph();
                let addresses = records
                    .iter()
                    .map(|reference| {
                        heap.heap_address_mut(*reference, REFERENCE_BYTES, REFERENCE_BYTES)
                            .expect("local field address should resolve")
                    })
                    .collect::<Vec<_>>();

                (heap, addresses)
            },
            |(heap, addresses)| {
                for pass in 0..FIELD_ACCESS_PASSES {
                    for (index, address) in addresses.iter().enumerate() {
                        // raw field store lower bound
                        unsafe {
                            std::ptr::write_unaligned((*address).cast::<usize>(), pass ^ index);
                        }
                    }
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_record_field_write_checked", |bencher| {
        bencher.iter_batched(
            || {
                let mut fixture = shared_fixture();
                let records = allocate_shared_object_graph(&fixture.heap, &mut fixture.allocator);

                (fixture.heap, records)
            },
            |(shared, records)| {
                for pass in 0..FIELD_ACCESS_PASSES {
                    for (index, reference) in records.iter().enumerate() {
                        let value = (pass ^ index).to_le_bytes();
                        shared
                            .write_heap_bytes(*reference, REFERENCE_BYTES, &value)
                            .expect("shared field write should succeed");
                    }
                }

                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_record_field_write_resolved", |bencher| {
        bencher.iter_batched(
            || {
                let mut fixture = shared_fixture();
                let records = allocate_shared_object_graph(&fixture.heap, &mut fixture.allocator);

                (fixture.heap, records)
            },
            |(shared, records)| {
                for pass in 0..FIELD_ACCESS_PASSES {
                    for (index, reference) in records.iter().enumerate() {
                        let address = shared
                            .heap_address_mut(*reference, REFERENCE_BYTES, REFERENCE_BYTES)
                            .expect("shared field address should resolve");

                        // field.addr plus raw store
                        unsafe {
                            std::ptr::write_unaligned(address.cast::<usize>(), pass ^ index);
                        }
                    }
                }

                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_record_field_write_direct", |bencher| {
        bencher.iter_batched(
            || {
                let mut fixture = shared_fixture();
                let records = allocate_shared_object_graph(&fixture.heap, &mut fixture.allocator);
                let addresses = records
                    .iter()
                    .map(|reference| {
                        fixture
                            .heap
                            .heap_address_mut(*reference, REFERENCE_BYTES, REFERENCE_BYTES)
                            .expect("shared field address should resolve")
                    })
                    .collect::<Vec<_>>();

                (fixture.heap, addresses)
            },
            |(shared, addresses)| {
                for pass in 0..FIELD_ACCESS_PASSES {
                    for (index, address) in addresses.iter().enumerate() {
                        // raw field store lower bound
                        unsafe {
                            std::ptr::write_unaligned((*address).cast::<usize>(), pass ^ index);
                        }
                    }
                }

                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}
