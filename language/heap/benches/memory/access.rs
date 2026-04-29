use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::common::{
    ACCESS_BYTES, ACCESS_PASSES, ACCESS_WORDS, REFERENCE_BYTES, SCALAR_ACCESS_PASSES,
    local_access_allocation, shared_access_allocation, source_bytes,
};

/// Benchmark direct heap access paths.
pub(crate) fn bench_heap_access(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_access");
    let total_bytes = (ACCESS_BYTES * ACCESS_PASSES) as u64;
    group.throughput(Throughput::Bytes(total_bytes));

    group.bench_function("local_bulk_read_checked", |bencher| {
        bencher.iter_batched(
            || {
                let payload = local_access_allocation();
                let buffer = vec![0u8; ACCESS_BYTES];

                (payload, buffer)
            },
            |((heap, reference), mut buffer)| {
                for _ in 0..ACCESS_PASSES {
                    heap.read_heap_bytes_into(reference, 0, black_box(&mut buffer))
                        .expect("local bulk read should succeed");
                }

                black_box(buffer);
                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_bulk_write_checked", |bencher| {
        bencher.iter_batched(
            || {
                let payload = local_access_allocation();
                let source = source_bytes();

                (payload, source)
            },
            |((mut heap, reference), source)| {
                for _ in 0..ACCESS_PASSES {
                    heap.write_heap_bytes(reference, 0, black_box(&source))
                        .expect("local bulk write should succeed");
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_bulk_read_direct", |bencher| {
        bencher.iter_batched(
            || {
                let payload = local_access_allocation();
                let buffer = vec![0u8; ACCESS_BYTES];

                (payload, buffer)
            },
            |((heap, reference), mut buffer)| {
                let address = heap
                    .heap_address(reference, 0, ACCESS_BYTES)
                    .expect("local bulk address should resolve");

                for _ in 0..ACCESS_PASSES {
                    // raw copy lower bound
                    unsafe {
                        std::ptr::copy_nonoverlapping(address, buffer.as_mut_ptr(), ACCESS_BYTES);
                    }
                }

                black_box(buffer);
                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_bulk_write_direct", |bencher| {
        bencher.iter_batched(
            || {
                let payload = local_access_allocation();
                let source = source_bytes();

                (payload, source)
            },
            |((mut heap, reference), source)| {
                let address = heap
                    .heap_address_mut(reference, 0, ACCESS_BYTES)
                    .expect("local bulk address should resolve");

                for _ in 0..ACCESS_PASSES {
                    // raw copy lower bound
                    unsafe {
                        std::ptr::copy_nonoverlapping(source.as_ptr(), address, ACCESS_BYTES);
                    }
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_bulk_read_checked", |bencher| {
        bencher.iter_batched(
            || {
                let payload = shared_access_allocation();
                let buffer = vec![0u8; ACCESS_BYTES];

                (payload, buffer)
            },
            |((shared, reference), mut buffer)| {
                for _ in 0..ACCESS_PASSES {
                    shared
                        .read_heap_bytes_into(reference, 0, black_box(&mut buffer))
                        .expect("shared bulk read should succeed");
                }

                black_box(buffer);
                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_bulk_write_checked", |bencher| {
        bencher.iter_batched(
            || {
                let payload = shared_access_allocation();
                let source = source_bytes();

                (payload, source)
            },
            |((shared, reference), source)| {
                for _ in 0..ACCESS_PASSES {
                    shared
                        .write_heap_bytes(reference, 0, black_box(&source))
                        .expect("shared bulk write should succeed");
                }

                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_bulk_read_direct", |bencher| {
        bencher.iter_batched(
            || {
                let payload = shared_access_allocation();
                let buffer = vec![0u8; ACCESS_BYTES];

                (payload, buffer)
            },
            |((shared, reference), mut buffer)| {
                let address = shared
                    .heap_address(reference, 0, ACCESS_BYTES)
                    .expect("shared bulk address should resolve");

                for _ in 0..ACCESS_PASSES {
                    // raw copy lower bound
                    unsafe {
                        std::ptr::copy_nonoverlapping(address, buffer.as_mut_ptr(), ACCESS_BYTES);
                    }
                }

                black_box(buffer);
                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_bulk_write_direct", |bencher| {
        bencher.iter_batched(
            || {
                let payload = shared_access_allocation();
                let source = source_bytes();

                (payload, source)
            },
            |((shared, reference), source)| {
                let address = shared
                    .heap_address_mut(reference, 0, ACCESS_BYTES)
                    .expect("shared bulk address should resolve");

                for _ in 0..ACCESS_PASSES {
                    // raw copy lower bound
                    unsafe {
                        std::ptr::copy_nonoverlapping(source.as_ptr(), address, ACCESS_BYTES);
                    }
                }

                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark scalar heap access paths.
pub(crate) fn bench_heap_scalar_access(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_scalar_access");
    let total_words = (ACCESS_WORDS * SCALAR_ACCESS_PASSES) as u64;
    group.throughput(Throughput::Elements(total_words));

    group.bench_function("local_word_read_checked", |bencher| {
        bencher.iter_batched(
            local_access_allocation,
            |(heap, reference)| {
                let mut buffer = [0u8; REFERENCE_BYTES];
                let mut sum = 0usize;

                for _ in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        let offset = word_index * REFERENCE_BYTES;
                        heap.read_heap_bytes_into(reference, offset, &mut buffer)
                            .expect("local word read should succeed");
                        sum = sum.wrapping_add(usize::from_le_bytes(buffer));
                    }
                }

                black_box(sum);
                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_word_write_checked", |bencher| {
        bencher.iter_batched(
            local_access_allocation,
            |(mut heap, reference)| {
                for pass in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        let offset = word_index * REFERENCE_BYTES;
                        let value = (pass ^ word_index).to_le_bytes();
                        heap.write_heap_bytes(reference, offset, &value)
                            .expect("local word write should succeed");
                    }
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_word_read_direct", |bencher| {
        bencher.iter_batched(
            local_access_allocation,
            |(heap, reference)| {
                let address = heap
                    .heap_address(reference, 0, ACCESS_BYTES)
                    .expect("local word address should resolve");
                let mut sum = 0usize;

                for _ in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        // raw word lower bound
                        let value = unsafe {
                            std::ptr::read_unaligned(address.cast::<usize>().add(word_index))
                        };
                        sum = sum.wrapping_add(value);
                    }
                }

                black_box(sum);
                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("local_word_write_direct", |bencher| {
        bencher.iter_batched(
            local_access_allocation,
            |(mut heap, reference)| {
                let address = heap
                    .heap_address_mut(reference, 0, ACCESS_BYTES)
                    .expect("local word address should resolve");

                for pass in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        // raw word lower bound
                        unsafe {
                            std::ptr::write_unaligned(
                                address.cast::<usize>().add(word_index),
                                pass ^ word_index,
                            );
                        }
                    }
                }

                black_box(heap);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_word_read_checked", |bencher| {
        bencher.iter_batched(
            shared_access_allocation,
            |(shared, reference)| {
                let mut buffer = [0u8; REFERENCE_BYTES];
                let mut sum = 0usize;

                for _ in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        let offset = word_index * REFERENCE_BYTES;
                        shared
                            .read_heap_bytes_into(reference, offset, &mut buffer)
                            .expect("shared word read should succeed");
                        sum = sum.wrapping_add(usize::from_le_bytes(buffer));
                    }
                }

                black_box(sum);
                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_word_write_checked", |bencher| {
        bencher.iter_batched(
            shared_access_allocation,
            |(shared, reference)| {
                for pass in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        let offset = word_index * REFERENCE_BYTES;
                        let value = (pass ^ word_index).to_le_bytes();
                        shared
                            .write_heap_bytes(reference, offset, &value)
                            .expect("shared word write should succeed");
                    }
                }

                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_word_read_direct", |bencher| {
        bencher.iter_batched(
            shared_access_allocation,
            |(shared, reference)| {
                let address = shared
                    .heap_address(reference, 0, ACCESS_BYTES)
                    .expect("shared word address should resolve");
                let mut sum = 0usize;

                for _ in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        // raw word lower bound
                        let value = unsafe {
                            std::ptr::read_unaligned(address.cast::<usize>().add(word_index))
                        };
                        sum = sum.wrapping_add(value);
                    }
                }

                black_box(sum);
                black_box(shared);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("shared_word_write_direct", |bencher| {
        bencher.iter_batched(
            shared_access_allocation,
            |(shared, reference)| {
                let address = shared
                    .heap_address_mut(reference, 0, ACCESS_BYTES)
                    .expect("shared word address should resolve");

                for pass in 0..SCALAR_ACCESS_PASSES {
                    for word_index in 0..ACCESS_WORDS {
                        // raw word lower bound
                        unsafe {
                            std::ptr::write_unaligned(
                                address.cast::<usize>().add(word_index),
                                pass ^ word_index,
                            );
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
