use std::hint::black_box;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput};
use destack_heap::AllocationShape;
use destack_mir::ReferenceMap;

use crate::config::{
    ALLOCATION_MATRIX_BYTES, LARGE_ALLOCATIONS, LARGE_BYTES, MATRIX_MAX_ALLOCATIONS,
    MATRIX_MIN_ALLOCATIONS, MATRIX_SAMPLE_BYTES, PARALLEL_ALLOCATIONS_PER_WORKER, PARALLEL_WORKERS,
    SMALL_ALLOCATIONS, SMALL_BYTES,
};
use crate::heap::{local_heap, shared_worker_heap};

/// Return the allocation count for one matrix size.
#[inline(always)]
fn matrix_allocation_count(byte_len: usize) -> usize {
    let count = MATRIX_SAMPLE_BYTES / byte_len;

    count.clamp(MATRIX_MIN_ALLOCATIONS, MATRIX_MAX_ALLOCATIONS)
}

/// Benchmark managed heap allocation paths.
pub(crate) fn bench_heap_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_allocation");
    let reference_map = ReferenceMap::None;
    let payload = [0xAB; SMALL_BYTES];
    let shape = AllocationShape::new(SMALL_BYTES, 1, &reference_map);
    let large_payload = vec![0xAB; LARGE_BYTES];
    let large_shape = AllocationShape::new(LARGE_BYTES, 1, &reference_map);

    // small objects exercise the cached allocator path
    group.throughput(Throughput::Elements(SMALL_ALLOCATIONS as u64));

    group.bench_function("local_small_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut heap = local_heap();
                let layout = heap.allocation_layout(shape);

                // prime the allocation run outside the timed loop
                let warm_reference = heap
                    .allocate_zeroed(&layout)
                    .expect("local heap allocation should prime");
                black_box(warm_reference);

                let start = Instant::now();

                // measure the public zeroed allocation path
                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = heap
                        .allocate_zeroed(&layout)
                        .expect("local heap allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(heap);
            }

            elapsed
        });
    });

    group.bench_function("local_small_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut heap = local_heap();
                let layout = heap.allocation_layout(shape);

                // prime the allocation run outside the timed loop
                let warm_reference = heap
                    .allocate_bytes(&layout, &payload)
                    .expect("local heap allocation should prime");
                black_box(warm_reference);

                let start = Instant::now();

                // measure the public initialized allocation path
                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = heap
                        .allocate_bytes(&layout, &payload)
                        .expect("local heap allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(heap);
            }

            elapsed
        });
    });

    group.bench_function("shared_small_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut shared_worker = shared_worker_heap();
                let layout = shared_worker.heap.allocation_layout(shape);

                // prime the allocation run outside the timed loop
                let warm_reference = shared_worker
                    .heap
                    .allocate_zeroed(&shared_worker.worker, &mut shared_worker.allocator, &layout)
                    .expect("shared heap allocation should prime");
                black_box(warm_reference);

                let start = Instant::now();

                // measure the shared zeroed allocation path with one worker cache
                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = shared_worker
                        .heap
                        .allocate_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            &layout,
                        )
                        .expect("shared heap allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(shared_worker);
            }

            elapsed
        });
    });

    group.bench_function("shared_small_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut shared_worker = shared_worker_heap();
                let layout = shared_worker.heap.allocation_layout(shape);

                // prime the allocation run outside the timed loop
                let warm_reference = shared_worker
                    .heap
                    .allocate_bytes(
                        &shared_worker.worker,
                        &mut shared_worker.allocator,
                        &layout,
                        &payload,
                    )
                    .expect("shared heap allocation should prime");
                black_box(warm_reference);

                let start = Instant::now();

                // measure the shared initialized allocation path with one worker cache
                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = shared_worker
                        .heap
                        .allocate_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            &layout,
                            &payload,
                        )
                        .expect("shared heap allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(shared_worker);
            }

            elapsed
        });
    });

    group.throughput(Throughput::Bytes((LARGE_ALLOCATIONS * LARGE_BYTES) as u64));

    // large objects exercise page-run allocation
    group.bench_function("local_large_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut heap = local_heap();
                let layout = heap.allocation_layout(large_shape);
                let start = Instant::now();

                // measure large zeroed allocations through the public path
                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = heap
                        .allocate_zeroed(&layout)
                        .expect("local large allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(heap);
            }

            elapsed
        });
    });

    group.bench_function("local_large_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut heap = local_heap();
                let layout = heap.allocation_layout(large_shape);
                let start = Instant::now();

                // measure large initialized allocations through the public path
                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = heap
                        .allocate_bytes(&layout, &large_payload)
                        .expect("local large allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(heap);
            }

            elapsed
        });
    });

    group.bench_function("shared_large_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut shared_worker = shared_worker_heap();
                let layout = shared_worker.heap.allocation_layout(large_shape);
                let start = Instant::now();

                // measure shared large zeroed allocations through one worker cache
                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = shared_worker
                        .heap
                        .allocate_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            &layout,
                        )
                        .expect("shared large allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(shared_worker);
            }

            elapsed
        });
    });

    group.bench_function("shared_large_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            // isolate heap setup from the timed allocation run
            for _ in 0..iterations {
                let mut shared_worker = shared_worker_heap();
                let layout = shared_worker.heap.allocation_layout(large_shape);
                let start = Instant::now();

                // measure shared large initialized allocations through one worker cache
                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = shared_worker
                        .heap
                        .allocate_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            &layout,
                            &large_payload,
                        )
                        .expect("shared large allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(shared_worker);
            }

            elapsed
        });
    });

    group.finish();
}

/// Benchmark managed allocation across size classes.
pub(crate) fn bench_heap_allocation_matrix(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_allocation_matrix");
    let reference_map = ReferenceMap::None;

    // sweep representative size classes through the public allocator paths
    for byte_len in ALLOCATION_MATRIX_BYTES {
        let allocation_count = matrix_allocation_count(*byte_len);
        group.throughput(Throughput::Bytes((*byte_len * allocation_count) as u64));

        group.bench_with_input(
            BenchmarkId::new("local_zeroed", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, &reference_map);

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    // isolate heap setup from the timed allocation run
                    for _ in 0..iterations {
                        let mut heap = local_heap();
                        let layout = heap.allocation_layout(shape);

                        // prime the allocation run outside the timed loop
                        let warm_reference = heap
                            .allocate_zeroed(&layout)
                            .expect("local zeroed allocation should prime");
                        black_box(warm_reference);

                        let start = Instant::now();

                        // measure local zeroed allocations at this size
                        for _ in 0..allocation_count {
                            let reference = heap
                                .allocate_zeroed(&layout)
                                .expect("local zeroed allocation should succeed");
                            black_box(reference);
                        }

                        elapsed += start.elapsed();
                        black_box(heap);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("local_bytes", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, &reference_map);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    // isolate heap setup from the timed allocation run
                    for _ in 0..iterations {
                        let mut heap = local_heap();
                        let layout = heap.allocation_layout(shape);

                        // prime the allocation run outside the timed loop
                        let warm_reference = heap
                            .allocate_bytes(&layout, &payload)
                            .expect("local byte allocation should prime");
                        black_box(warm_reference);

                        let start = Instant::now();

                        // measure local initialized allocations at this size
                        for _ in 0..allocation_count {
                            let reference = heap
                                .allocate_bytes(&layout, &payload)
                                .expect("local byte allocation should succeed");
                            black_box(reference);
                        }

                        elapsed += start.elapsed();
                        black_box(heap);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("shared_zeroed", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, &reference_map);

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    // isolate heap setup from the timed allocation run
                    for _ in 0..iterations {
                        let mut shared_worker = shared_worker_heap();
                        let layout = shared_worker.heap.allocation_layout(shape);

                        // prime the allocation run outside the timed loop
                        let warm_reference = shared_worker
                            .heap
                            .allocate_zeroed(
                                &shared_worker.worker,
                                &mut shared_worker.allocator,
                                &layout,
                            )
                            .expect("shared zeroed allocation should prime");
                        black_box(warm_reference);

                        let start = Instant::now();

                        // measure shared zeroed allocations at this size
                        for _ in 0..allocation_count {
                            let reference = shared_worker
                                .heap
                                .allocate_zeroed(
                                    &shared_worker.worker,
                                    &mut shared_worker.allocator,
                                    &layout,
                                )
                                .expect("shared zeroed allocation should succeed");
                            black_box(reference);
                        }

                        elapsed += start.elapsed();
                        black_box(shared_worker);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("shared_bytes", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, &reference_map);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    // isolate heap setup from the timed allocation run
                    for _ in 0..iterations {
                        let mut shared_worker = shared_worker_heap();
                        let layout = shared_worker.heap.allocation_layout(shape);

                        // prime the allocation run outside the timed loop
                        let warm_reference = shared_worker
                            .heap
                            .allocate_bytes(
                                &shared_worker.worker,
                                &mut shared_worker.allocator,
                                &layout,
                                &payload,
                            )
                            .expect("shared byte allocation should prime");
                        black_box(warm_reference);

                        let start = Instant::now();

                        // measure shared initialized allocations at this size
                        for _ in 0..allocation_count {
                            let reference = shared_worker
                                .heap
                                .allocate_bytes(
                                    &shared_worker.worker,
                                    &mut shared_worker.allocator,
                                    &layout,
                                    &payload,
                                )
                                .expect("shared byte allocation should succeed");
                            black_box(reference);
                        }

                        elapsed += start.elapsed();
                        black_box(shared_worker);
                    }

                    elapsed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark shared heap allocation with several worker-local allocators.
pub(crate) fn bench_shared_parallel_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_shared_parallel_allocation");
    let reference_map = ReferenceMap::None;
    let shape = AllocationShape::new(SMALL_BYTES, 1, &reference_map);
    group.throughput(Throughput::Elements(
        (PARALLEL_ALLOCATIONS_PER_WORKER * PARALLEL_WORKERS[0]) as u64,
    ));

    // scale shared allocation across worker-local allocator caches
    for worker_count in PARALLEL_WORKERS {
        group.throughput(Throughput::Elements(
            (PARALLEL_ALLOCATIONS_PER_WORKER * *worker_count) as u64,
        ));

        group.bench_with_input(
            BenchmarkId::new("small_zeroed", worker_count),
            worker_count,
            |bencher, worker_count| {
                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    // isolate shared heap setup from the timed worker run
                    for _ in 0..iterations {
                        let shared_worker = shared_worker_heap();
                        let shared = Arc::new(shared_worker.heap);
                        let layout = shared.allocation_layout(shape);
                        let barrier = Arc::new(Barrier::new(*worker_count + 1));

                        let start = Instant::now();
                        thread::scope(|scope| {
                            // each worker allocates from its own shared allocator cache
                            for _ in 0..*worker_count {
                                let shared = Arc::clone(&shared);
                                let barrier = Arc::clone(&barrier);

                                scope.spawn(move || {
                                    let mut allocator = shared.allocator();
                                    let worker = shared.register_collector_worker();
                                    barrier.wait();

                                    // measure the parallel public allocation path
                                    for _ in 0..PARALLEL_ALLOCATIONS_PER_WORKER {
                                        let reference = shared
                                            .allocate_zeroed(&worker, &mut allocator, &layout)
                                            .expect("shared parallel allocation should succeed");
                                        black_box(reference);
                                    }

                                    black_box(allocator);
                                });
                            }

                            // release all workers after setup
                            barrier.wait();
                        });
                        elapsed += start.elapsed();

                        black_box(shared);
                    }

                    elapsed
                });
            },
        );
    }

    group.finish();
}
