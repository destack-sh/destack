use std::hint::black_box;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput};
use destack_heap::{
    AllocationShape, AllocationSite, Heap, SmallAllocationPlan, SmallAllocationSite,
};
use destack_mir::{TraceMap, TraceTable};

use crate::config::{
    ALLOCATION_MATRIX_BYTES, LARGE_ALLOCATIONS, LARGE_BYTES, MATRIX_MAX_ALLOCATIONS,
    MATRIX_MIN_ALLOCATIONS, MATRIX_SAMPLE_BYTES, PARALLEL_ALLOCATIONS_PER_WORKER, PARALLEL_WORKERS,
    SMALL_ALLOCATIONS, SMALL_BYTES,
};
use crate::heap::{WorkerHeap, local_heap, shared_heap, shared_worker_heap};

/// Benchmark managed heap allocation paths.
pub(crate) fn bench_heap_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_allocation");

    // build trace maps used by the specialized small paths
    let trace_map = TraceMap::Empty;
    let mut trace_table = TraceTable::new();
    let local_trace_map = local_reference_trace_map();
    let shared_trace_map = shared_reference_trace_map();
    let local_trace_id = trace_table.insert(local_trace_map.clone());
    let shared_trace_id = trace_table.insert(shared_trace_map.clone());

    // derive allocation sites from one representative heap
    let heap = local_heap();
    let shape = AllocationShape::new(SMALL_BYTES, 1, None, &trace_map);
    let allocation = heap.allocation_site(shape);
    let small_site = small_allocation_site(allocation);
    let small = small_allocation(allocation);
    let local_shape = AllocationShape::new(SMALL_BYTES, 1, Some(local_trace_id), &local_trace_map);
    let local_allocation = heap.allocation_site(local_shape);
    let local_small_site = small_allocation_site(local_allocation);
    let shared_shape =
        AllocationShape::new(SMALL_BYTES, 1, Some(shared_trace_id), &shared_trace_map);
    let shared_allocation = heap.allocation_site(shared_shape);
    let shared_small_site = small_allocation_site(shared_allocation);

    // build initialized payloads for byte-copy paths
    let payload = [0xAB; SMALL_BYTES];
    let large_payload = vec![0xAB; LARGE_BYTES];
    let large_shape = AllocationShape::new(LARGE_BYTES, 1, None, &trace_map);

    // measure small cached allocator paths
    group.throughput(Throughput::Elements(SMALL_ALLOCATIONS as u64));

    group.bench_function("local_small_noscan_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |heap| {
                    let reference = heap
                        .allocate_zeroed(allocation, &trace_map)
                        .expect("local noscan allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .reserve_small_noscan(small_site)
                        .expect("local noscan allocation should stay hot");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("local_small_scan_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |heap| {
                    let reference = heap
                        .allocate_zeroed(local_allocation, &local_trace_map)
                        .expect("local scan allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .reserve_small_scan(local_small_site)
                        .expect("local scan allocation should stay hot");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("local_small_shared_edge_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |heap| {
                    let reference = heap
                        .allocate_zeroed(shared_allocation, &shared_trace_map)
                        .expect("local shared-edge allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .reserve_small_shared_edge(shared_small_site)
                        .expect("local shared-edge allocation should stay hot");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("local_small_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |heap| {
                    let reference = heap
                        .allocate_dynamic_zeroed(shape)
                        .expect("local heap allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .allocate_dynamic_zeroed(shape)
                        .expect("local heap allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("local_small_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |heap| {
                    let reference = heap
                        .allocate_dynamic_bytes(shape, &payload)
                        .expect("local heap allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .allocate_dynamic_bytes(shape, &payload)
                        .expect("local heap allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("shared_small_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_shared_worker_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_dynamic_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            shape,
                            &trace_table,
                        )
                        .expect("shared heap allocation should prime");
                    black_box(reference);
                },
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_dynamic_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            shape,
                            &trace_table,
                        )
                        .expect("shared heap allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("shared_small_noscan_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_shared_worker_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            allocation,
                            &trace_map,
                            &trace_table,
                        )
                        .expect("shared noscan allocation should prime");
                    black_box(reference);
                },
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .reserve_small_from_cache(&mut shared_worker.allocator, small)
                        .expect("shared noscan allocation should stay hot");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("shared_small_size_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_shared_worker_heap(
                iterations,
                SMALL_ALLOCATIONS,
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_dynamic_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            shape,
                            &payload,
                            &trace_table,
                        )
                        .expect("shared heap allocation should prime");
                    black_box(reference);
                },
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_dynamic_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            shape,
                            &payload,
                            &trace_table,
                        )
                        .expect("shared heap allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    // measure large page-run allocation paths
    group.throughput(Throughput::Bytes((LARGE_ALLOCATIONS * LARGE_BYTES) as u64));

    group.bench_function("local_large_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                LARGE_ALLOCATIONS,
                |_| {},
                |heap| {
                    let reference = heap
                        .allocate_dynamic_zeroed(large_shape)
                        .expect("local large allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("local_large_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_local_heap(
                iterations,
                LARGE_ALLOCATIONS,
                |_| {},
                |heap| {
                    let reference = heap
                        .allocate_dynamic_bytes(large_shape, &large_payload)
                        .expect("local large allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("shared_large_zeroed", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_shared_worker_heap(
                iterations,
                LARGE_ALLOCATIONS,
                |_| {},
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_dynamic_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            large_shape,
                            &trace_table,
                        )
                        .expect("shared large allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.bench_function("shared_large_bytes", |bencher| {
        bencher.iter_custom(|iterations| {
            measure_shared_worker_heap(
                iterations,
                LARGE_ALLOCATIONS,
                |_| {},
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_dynamic_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.allocator,
                            large_shape,
                            &large_payload,
                            &trace_table,
                        )
                        .expect("shared large allocation should succeed");
                    black_box(reference);
                },
            )
        });
    });

    group.finish();
}

/// Benchmark managed allocation across size classes.
pub(crate) fn bench_heap_allocation_matrix(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_allocation_matrix");
    let trace_map = TraceMap::Empty;
    let trace_table = TraceTable::new();

    // sweep representative size classes through dynamic allocator paths
    for byte_len in ALLOCATION_MATRIX_BYTES {
        let allocation_count = matrix_allocation_count(*byte_len);
        group.throughput(Throughput::Bytes((*byte_len * allocation_count) as u64));

        group.bench_with_input(
            BenchmarkId::new("local_zeroed", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, None, &trace_map);

                bencher.iter_custom(|iterations| {
                    measure_local_heap(
                        iterations,
                        allocation_count,
                        |heap| {
                            let reference = heap
                                .allocate_dynamic_zeroed(shape)
                                .expect("local zeroed allocation should prime");
                            black_box(reference);
                        },
                        |heap| {
                            let reference = heap
                                .allocate_dynamic_zeroed(shape)
                                .expect("local zeroed allocation should succeed");
                            black_box(reference);
                        },
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("local_bytes", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, None, &trace_map);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    measure_local_heap(
                        iterations,
                        allocation_count,
                        |heap| {
                            let reference = heap
                                .allocate_dynamic_bytes(shape, &payload)
                                .expect("local byte allocation should prime");
                            black_box(reference);
                        },
                        |heap| {
                            let reference = heap
                                .allocate_dynamic_bytes(shape, &payload)
                                .expect("local byte allocation should succeed");
                            black_box(reference);
                        },
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("shared_zeroed", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, None, &trace_map);

                bencher.iter_custom(|iterations| {
                    measure_shared_worker_heap(
                        iterations,
                        allocation_count,
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_dynamic_zeroed(
                                    &shared_worker.worker,
                                    &mut shared_worker.allocator,
                                    shape,
                                    &trace_table,
                                )
                                .expect("shared zeroed allocation should prime");
                            black_box(reference);
                        },
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_dynamic_zeroed(
                                    &shared_worker.worker,
                                    &mut shared_worker.allocator,
                                    shape,
                                    &trace_table,
                                )
                                .expect("shared zeroed allocation should succeed");
                            black_box(reference);
                        },
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("shared_bytes", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, None, &trace_map);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    measure_shared_worker_heap(
                        iterations,
                        allocation_count,
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_dynamic_bytes(
                                    &shared_worker.worker,
                                    &mut shared_worker.allocator,
                                    shape,
                                    &payload,
                                    &trace_table,
                                )
                                .expect("shared byte allocation should prime");
                            black_box(reference);
                        },
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_dynamic_bytes(
                                    &shared_worker.worker,
                                    &mut shared_worker.allocator,
                                    shape,
                                    &payload,
                                    &trace_table,
                                )
                                .expect("shared byte allocation should succeed");
                            black_box(reference);
                        },
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark shared heap allocation with several worker-local allocators.
pub(crate) fn bench_shared_parallel_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_shared_parallel_allocation");
    let trace_map = TraceMap::Empty;
    let trace_table = TraceTable::new();
    let shape = AllocationShape::new(SMALL_BYTES, 1, None, &trace_map);

    // scale shared allocation across worker-local allocator caches
    for worker_count in PARALLEL_WORKERS {
        let allocation_count = PARALLEL_ALLOCATIONS_PER_WORKER * *worker_count;
        group.throughput(Throughput::Elements(allocation_count as u64));

        group.bench_with_input(
            BenchmarkId::new("small_zeroed", worker_count),
            worker_count,
            |bencher, worker_count| {
                bencher.iter_custom(|iterations| {
                    measure_parallel_shared_heap(iterations, *worker_count, shape, &trace_table)
                });
            },
        );
    }

    group.finish();
}

/// Measure repeated local heap allocations with setup outside the timed loop.
fn measure_local_heap(
    iterations: u64,
    allocation_count: usize,
    mut prepare: impl FnMut(&mut Heap),
    mut allocate: impl FnMut(&mut Heap),
) -> Duration {
    let mut elapsed = Duration::ZERO;

    // rebuild the heap for each criterion iteration
    for _ in 0..iterations {
        let mut heap = local_heap();
        prepare(&mut heap);

        let start = Instant::now();

        // run only the target allocation path
        for _ in 0..allocation_count {
            allocate(&mut heap);
        }

        elapsed += start.elapsed();
        black_box(heap);
    }

    elapsed
}

/// Measure repeated shared heap allocations with setup outside the timed loop.
fn measure_shared_worker_heap(
    iterations: u64,
    allocation_count: usize,
    mut prepare: impl FnMut(&mut WorkerHeap),
    mut allocate: impl FnMut(&mut WorkerHeap),
) -> Duration {
    let mut elapsed = Duration::ZERO;

    // rebuild the shared heap for each criterion iteration
    for _ in 0..iterations {
        let mut shared_worker = shared_worker_heap();
        prepare(&mut shared_worker);

        let start = Instant::now();

        // run only the target allocation path
        for _ in 0..allocation_count {
            allocate(&mut shared_worker);
        }

        elapsed += start.elapsed();
        black_box(shared_worker);
    }

    elapsed
}

/// Measure parallel shared allocations after worker setup has completed.
fn measure_parallel_shared_heap(
    iterations: u64,
    worker_count: usize,
    shape: AllocationShape<'_>,
    trace_table: &TraceTable,
) -> Duration {
    let mut elapsed = Duration::ZERO;

    // rebuild the shared heap for each criterion iteration
    for _ in 0..iterations {
        let shared = Arc::new(shared_heap());
        let ready = Arc::new(Barrier::new(worker_count + 1));
        let release = Arc::new(Barrier::new(worker_count + 1));

        let started_at = thread::scope(|scope| {
            // prepare each worker before timing starts
            for _ in 0..worker_count {
                let shared = Arc::clone(&shared);
                let ready = Arc::clone(&ready);
                let release = Arc::clone(&release);

                scope.spawn(move || {
                    let mut allocator = shared.allocation_cache();
                    let worker = shared.register_collector_worker();
                    ready.wait();
                    release.wait();

                    // allocate through one worker-local cache
                    for _ in 0..PARALLEL_ALLOCATIONS_PER_WORKER {
                        let reference = shared
                            .allocate_dynamic_zeroed(&worker, &mut allocator, shape, trace_table)
                            .expect("shared parallel allocation should succeed");
                        black_box(reference);
                    }

                    black_box(allocator);
                });
            }

            ready.wait();
            let started_at = Instant::now();
            release.wait();

            started_at
        });

        elapsed += started_at.elapsed();
        black_box(shared);
    }

    elapsed
}

/// Return the allocation count for one matrix size.
#[inline(always)]
fn matrix_allocation_count(byte_len: usize) -> usize {
    let count = MATRIX_SAMPLE_BYTES / byte_len;

    count.clamp(MATRIX_MIN_ALLOCATIONS, MATRIX_MAX_ALLOCATIONS)
}

/// Return the small allocation plan for one allocation site.
#[inline(always)]
fn small_allocation(allocation: AllocationSite) -> SmallAllocationPlan {
    small_allocation_site(allocation).small
}

/// Return the small allocation site for one allocation site.
#[inline(always)]
fn small_allocation_site(allocation: AllocationSite) -> SmallAllocationSite {
    allocation
        .small_site()
        .expect("allocation should use a small class")
}

/// Return one trace map with one local reference word.
fn local_reference_trace_map() -> TraceMap {
    TraceMap::Fixed {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
    }
}

/// Return one trace map with one shared reference word.
fn shared_reference_trace_map() -> TraceMap {
    TraceMap::Fixed {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
    }
}
