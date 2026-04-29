use std::hint::black_box;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput};
use destack_heap::AllocationLayout;
use destack_mir::ReferenceMap;

use crate::common::{
    ALLOCATION_MATRIX_BYTES, LARGE_ALLOCATIONS, LARGE_BYTES, MATRIX_ALLOCATIONS,
    PARALLEL_ALLOCATIONS_PER_WORKER, PARALLEL_WORKERS, SMALL_ALLOCATIONS, SMALL_BYTES, local_heap,
    shared_fixture,
};

/// Benchmark managed heap allocation paths.
pub(crate) fn bench_heap_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_allocation");
    let reference_map = ReferenceMap::None;
    let payload = [0xAB; SMALL_BYTES];
    let layout = AllocationLayout::new(SMALL_BYTES, &reference_map);
    let large_payload = vec![0xAB; LARGE_BYTES];
    let large_layout = AllocationLayout::new(LARGE_BYTES, &reference_map);

    group.throughput(Throughput::Elements(SMALL_ALLOCATIONS as u64));

    group.bench_function("local_small_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            for _ in 0..iterations {
                let mut heap = local_heap();
                let start = Instant::now();

                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = heap
                        .allocate_zeroed(layout)
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

            for _ in 0..iterations {
                let mut heap = local_heap();
                let start = Instant::now();

                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = heap
                        .allocate_bytes(layout, &payload)
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

            for _ in 0..iterations {
                let mut fixture = shared_fixture();
                let start = Instant::now();

                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = fixture
                        .heap
                        .allocate_zeroed(&mut fixture.allocator, layout)
                        .expect("shared heap allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(fixture);
            }

            elapsed
        });
    });

    group.bench_function("shared_small_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            for _ in 0..iterations {
                let mut fixture = shared_fixture();
                let start = Instant::now();

                for _ in 0..SMALL_ALLOCATIONS {
                    let reference = fixture
                        .heap
                        .allocate_bytes(&mut fixture.allocator, layout, &payload)
                        .expect("shared heap allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(fixture);
            }

            elapsed
        });
    });

    group.throughput(Throughput::Bytes((LARGE_ALLOCATIONS * LARGE_BYTES) as u64));

    group.bench_function("local_large_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            for _ in 0..iterations {
                let mut heap = local_heap();
                let start = Instant::now();

                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = heap
                        .allocate_zeroed(large_layout)
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

            for _ in 0..iterations {
                let mut heap = local_heap();
                let start = Instant::now();

                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = heap
                        .allocate_bytes(large_layout, &large_payload)
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

            for _ in 0..iterations {
                let mut fixture = shared_fixture();
                let start = Instant::now();

                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = fixture
                        .heap
                        .allocate_zeroed(&mut fixture.allocator, large_layout)
                        .expect("shared large allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(fixture);
            }

            elapsed
        });
    });

    group.bench_function("shared_large_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            for _ in 0..iterations {
                let mut fixture = shared_fixture();
                let start = Instant::now();

                for _ in 0..LARGE_ALLOCATIONS {
                    let reference = fixture
                        .heap
                        .allocate_bytes(&mut fixture.allocator, large_layout, &large_payload)
                        .expect("shared large allocation should succeed");
                    black_box(reference);
                }

                elapsed += start.elapsed();
                black_box(fixture);
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

    for byte_len in ALLOCATION_MATRIX_BYTES {
        group.throughput(Throughput::Bytes((*byte_len * MATRIX_ALLOCATIONS) as u64));

        group.bench_with_input(
            BenchmarkId::new("local_zeroed", byte_len),
            byte_len,
            |bencher, byte_len| {
                let layout = AllocationLayout::new(*byte_len, &reference_map);
                let plan = layout.plan();

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let mut heap = local_heap();
                        let start = Instant::now();

                        for _ in 0..MATRIX_ALLOCATIONS {
                            let reference = heap
                                .allocate_plan_zeroed(plan)
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
                let layout = AllocationLayout::new(*byte_len, &reference_map);
                let plan = layout.plan();
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let mut heap = local_heap();
                        let start = Instant::now();

                        for _ in 0..MATRIX_ALLOCATIONS {
                            let reference = heap
                                .allocate_plan_bytes(plan, &payload)
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
                let layout = AllocationLayout::new(*byte_len, &reference_map);

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let mut fixture = shared_fixture();
                        let plan = layout.plan();
                        let plan =
                            plan.with_small_span_class(fixture.allocator.small_span_class(plan));
                        let start = Instant::now();

                        for _ in 0..MATRIX_ALLOCATIONS {
                            let reference = fixture
                                .heap
                                .allocate_plan_zeroed_for_worker(None, &mut fixture.allocator, plan)
                                .expect("shared zeroed allocation should succeed");
                            black_box(reference);
                        }

                        elapsed += start.elapsed();
                        black_box(fixture);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("shared_bytes", byte_len),
            byte_len,
            |bencher, byte_len| {
                let layout = AllocationLayout::new(*byte_len, &reference_map);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let mut fixture = shared_fixture();
                        let plan = layout.plan();
                        let plan =
                            plan.with_small_span_class(fixture.allocator.small_span_class(plan));
                        let start = Instant::now();

                        for _ in 0..MATRIX_ALLOCATIONS {
                            let reference = fixture
                                .heap
                                .allocate_plan_bytes_for_worker(
                                    None,
                                    &mut fixture.allocator,
                                    plan,
                                    &payload,
                                )
                                .expect("shared byte allocation should succeed");
                            black_box(reference);
                        }

                        elapsed += start.elapsed();
                        black_box(fixture);
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
    let layout = AllocationLayout::new(SMALL_BYTES, &reference_map);
    group.throughput(Throughput::Elements(
        (PARALLEL_ALLOCATIONS_PER_WORKER * PARALLEL_WORKERS[0]) as u64,
    ));

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

                    for _ in 0..iterations {
                        let fixture = shared_fixture();
                        let shared = Arc::new(fixture.heap);
                        let barrier = Arc::new(Barrier::new(*worker_count + 1));

                        let start = Instant::now();
                        std::thread::scope(|scope| {
                            for _ in 0..*worker_count {
                                let shared = Arc::clone(&shared);
                                let barrier = Arc::clone(&barrier);

                                scope.spawn(move || {
                                    let mut allocator = shared.allocator();
                                    barrier.wait();

                                    for _ in 0..PARALLEL_ALLOCATIONS_PER_WORKER {
                                        let reference = shared
                                            .allocate_zeroed(&mut allocator, layout)
                                            .expect("shared parallel allocation should succeed");
                                        black_box(reference);
                                    }

                                    black_box(allocator);
                                });
                            }

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
