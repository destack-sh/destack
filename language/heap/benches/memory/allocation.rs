use std::hint::black_box;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput};
use tspp_heap::{
    AllocationPlan, AllocationShape, Heap, SharedHeap, SmallAllocationClass, SmallAllocationPlan,
    TraceView,
};
use tspp_mir::TraceMap;

use crate::config::{
    ALLOCATION_MATRIX_BYTES, LARGE_ALLOCATIONS, LARGE_BYTES, MATRIX_MAX_ALLOCATIONS,
    MATRIX_MIN_ALLOCATIONS, MATRIX_SAMPLE_BYTES, PARALLEL_ALLOCATIONS_PER_WORKER, PARALLEL_WORKERS,
    SMALL_ALLOCATIONS, SMALL_BYTES,
};
use crate::heap::{WorkerHeap, local_heap, shared_heap, shared_worker_heap};
use crate::trace::BenchTraceTable;

/// Benchmark managed heap allocation paths.
pub(crate) fn bench_heap_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_allocation");

    // build trace maps used by the specialized small paths
    let trace_map = TraceMap::Empty;
    let mut source_traces = tspp_mir::TraceTable::new();
    let local_trace_map = local_reference_trace_map();
    let shared_trace_map = shared_reference_trace_map();
    let local_trace_id = source_traces.insert(local_trace_map.clone());
    let shared_trace_id = source_traces.insert(shared_trace_map.clone());
    let trace_table = BenchTraceTable::from_mir(&source_traces);
    let trace_view = trace_table.view();

    // derive allocation plans from one representative heap
    let heap = local_heap();
    let shared_heap = shared_heap();
    let shape = AllocationShape::new(SMALL_BYTES, 1, None, trace_map.clone());
    let allocation = local_allocation_plan(&heap, &shape);
    let small_site = small_allocation_plan(allocation);
    let shared_allocation = shared_allocation_plan(&shared_heap, &shape);
    let shared_small = small_allocation(shared_allocation);
    let local_shape = AllocationShape::new(SMALL_BYTES, 1, Some(local_trace_id), local_trace_map);
    let local_allocation = local_allocation_plan(&heap, &local_shape);
    let local_small_site = small_allocation_plan(local_allocation);
    let shared_edge_shape =
        AllocationShape::new(SMALL_BYTES, 1, Some(shared_trace_id), shared_trace_map);
    let shared_edge_allocation = local_allocation_plan(&heap, &shared_edge_shape);
    let shared_edge_small_site = small_allocation_plan(shared_edge_allocation);

    // build initialized payloads for byte-copy paths
    let payload = [0xAB; SMALL_BYTES];
    let large_payload = vec![0xAB; LARGE_BYTES];
    let large_shape = AllocationShape::new(LARGE_BYTES, 1, None, trace_map.clone());
    let large_allocation = local_allocation_plan(&heap, &large_shape);
    let shared_large_allocation = shared_allocation_plan(&shared_heap, &large_shape);

    // measure small cached allocation paths
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
                        .reserve_small(small_site)
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
                        .allocate_zeroed(local_allocation, &local_shape.trace_map)
                        .expect("local scan allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .reserve_small(local_small_site)
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
                        .allocate_zeroed(shared_edge_allocation, &shared_edge_shape.trace_map)
                        .expect("local shared-edge allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .reserve_small_shared_edge(shared_edge_small_site)
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
                        .allocate_zeroed(allocation, &trace_map)
                        .expect("local heap allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .allocate_zeroed(allocation, &trace_map)
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
                        .allocate_bytes(allocation, &trace_map, &payload)
                        .expect("local heap allocation should prime");
                    black_box(reference);
                },
                |heap| {
                    let reference = heap
                        .allocate_bytes(allocation, &trace_map, &payload)
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
                        .allocate_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.cache,
                            shared_allocation,
                            &trace_map,
                            trace_view,
                        )
                        .expect("shared heap allocation should prime");
                    black_box(reference);
                },
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.cache,
                            shared_allocation,
                            &trace_map,
                            trace_view,
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
                            &mut shared_worker.cache,
                            shared_allocation,
                            &trace_map,
                            trace_view,
                        )
                        .expect("shared noscan allocation should prime");
                    black_box(reference);
                },
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .reserve_small_from_cache(&mut shared_worker.cache, shared_small)
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
                        .allocate_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.cache,
                            shared_allocation,
                            &trace_map,
                            &payload,
                            trace_view,
                        )
                        .expect("shared heap allocation should prime");
                    black_box(reference);
                },
                |shared_worker| {
                    let reference = shared_worker
                        .heap
                        .allocate_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.cache,
                            shared_allocation,
                            &trace_map,
                            &payload,
                            trace_view,
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
                        .allocate_zeroed(large_allocation, &trace_map)
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
                        .allocate_bytes(large_allocation, &trace_map, &large_payload)
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
                        .allocate_zeroed(
                            &shared_worker.worker,
                            &mut shared_worker.cache,
                            shared_large_allocation,
                            &trace_map,
                            trace_view,
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
                        .allocate_bytes(
                            &shared_worker.worker,
                            &mut shared_worker.cache,
                            shared_large_allocation,
                            &trace_map,
                            &large_payload,
                            trace_view,
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
    let trace_table = BenchTraceTable::new();
    let trace_view = trace_table.view();

    // sweep representative size classes through dynamic allocation paths
    for byte_len in ALLOCATION_MATRIX_BYTES {
        let allocation_count = matrix_allocation_count(*byte_len);
        group.throughput(Throughput::Bytes((*byte_len * allocation_count) as u64));

        group.bench_with_input(
            BenchmarkId::new("local_zeroed", byte_len),
            byte_len,
            |bencher, byte_len| {
                let shape = AllocationShape::new(*byte_len, 1, None, TraceMap::Empty);
                let heap = local_heap();
                let allocation = local_allocation_plan(&heap, &shape);

                bencher.iter_custom(|iterations| {
                    measure_local_heap(
                        iterations,
                        allocation_count,
                        |heap| {
                            let reference = heap
                                .allocate_zeroed(allocation, &shape.trace_map)
                                .expect("local zeroed allocation should prime");
                            black_box(reference);
                        },
                        |heap| {
                            let reference = heap
                                .allocate_zeroed(allocation, &shape.trace_map)
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
                let shape = AllocationShape::new(*byte_len, 1, None, TraceMap::Empty);
                let heap = local_heap();
                let allocation = local_allocation_plan(&heap, &shape);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    measure_local_heap(
                        iterations,
                        allocation_count,
                        |heap| {
                            let reference = heap
                                .allocate_bytes(allocation, &shape.trace_map, &payload)
                                .expect("local byte allocation should prime");
                            black_box(reference);
                        },
                        |heap| {
                            let reference = heap
                                .allocate_bytes(allocation, &shape.trace_map, &payload)
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
                let shape = AllocationShape::new(*byte_len, 1, None, TraceMap::Empty);
                let shared = shared_heap();
                let allocation = shared_allocation_plan(&shared, &shape);

                bencher.iter_custom(|iterations| {
                    measure_shared_worker_heap(
                        iterations,
                        allocation_count,
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_zeroed(
                                    &shared_worker.worker,
                                    &mut shared_worker.cache,
                                    allocation,
                                    &shape.trace_map,
                                    trace_view,
                                )
                                .expect("shared zeroed allocation should prime");
                            black_box(reference);
                        },
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_zeroed(
                                    &shared_worker.worker,
                                    &mut shared_worker.cache,
                                    allocation,
                                    &shape.trace_map,
                                    trace_view,
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
                let shape = AllocationShape::new(*byte_len, 1, None, TraceMap::Empty);
                let shared = shared_heap();
                let allocation = shared_allocation_plan(&shared, &shape);
                let payload = vec![0xAB; *byte_len];

                bencher.iter_custom(|iterations| {
                    measure_shared_worker_heap(
                        iterations,
                        allocation_count,
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_bytes(
                                    &shared_worker.worker,
                                    &mut shared_worker.cache,
                                    allocation,
                                    &shape.trace_map,
                                    &payload,
                                    trace_view,
                                )
                                .expect("shared byte allocation should prime");
                            black_box(reference);
                        },
                        |shared_worker| {
                            let reference = shared_worker
                                .heap
                                .allocate_bytes(
                                    &shared_worker.worker,
                                    &mut shared_worker.cache,
                                    allocation,
                                    &shape.trace_map,
                                    &payload,
                                    trace_view,
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

/// Benchmark shared heap allocation with several worker-local caches.
pub(crate) fn bench_shared_parallel_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_shared_parallel_allocation");
    let trace_map = TraceMap::Empty;
    let trace_table = BenchTraceTable::new();
    let trace_view = trace_table.view();
    let shape = AllocationShape::new(SMALL_BYTES, 1, None, trace_map.clone());
    let shared = shared_heap();
    let allocation = shared_allocation_plan(&shared, &shape);

    // scale shared allocation across worker-local caches
    for worker_count in PARALLEL_WORKERS {
        let allocation_count = PARALLEL_ALLOCATIONS_PER_WORKER * *worker_count;
        group.throughput(Throughput::Elements(allocation_count as u64));

        group.bench_with_input(
            BenchmarkId::new("small_zeroed", worker_count),
            worker_count,
            |bencher, worker_count| {
                bencher.iter_custom(|iterations| {
                    measure_parallel_shared_heap(
                        iterations,
                        *worker_count,
                        allocation,
                        &trace_map,
                        trace_view,
                    )
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
    allocation: AllocationPlan,
    trace_map: &TraceMap,
    trace_view: TraceView<'_>,
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
                    let mut cache = shared.allocation_cache();
                    let worker = shared.register_mark_worker();
                    ready.wait();
                    release.wait();

                    // allocate through one worker-local cache
                    for _ in 0..PARALLEL_ALLOCATIONS_PER_WORKER {
                        let reference = shared
                            .allocate_zeroed(&worker, &mut cache, allocation, trace_map, trace_view)
                            .expect("shared parallel allocation should succeed");
                        black_box(reference);
                    }

                    black_box(cache);
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

/// Return the small allocation class for one allocation plan.
#[inline(always)]
fn small_allocation(allocation: AllocationPlan) -> SmallAllocationClass {
    small_allocation_plan(allocation).small
}

/// Return the small allocation plan for one allocation plan.
#[inline(always)]
fn small_allocation_plan(allocation: AllocationPlan) -> SmallAllocationPlan {
    allocation
        .small_allocation()
        .expect("allocation should use a small class")
}

/// Build one explicit local allocation plan for benchmarks.
#[inline(always)]
fn local_allocation_plan(heap: &Heap, shape: &AllocationShape) -> AllocationPlan {
    heap.options().allocation_plan(shape)
}

/// Build one explicit shared allocation plan for benchmarks.
#[inline(always)]
fn shared_allocation_plan(heap: &SharedHeap, shape: &AllocationShape) -> AllocationPlan {
    heap.options().allocation_plan(shape)
}

/// Return one trace map with one local reference word.
fn local_reference_trace_map() -> TraceMap {
    TraceMap::Fixed {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
        frame_offsets: Box::new([]),
        borrow_offsets: Box::default(),
    }
}

/// Return one trace map with one shared reference word.
fn shared_reference_trace_map() -> TraceMap {
    TraceMap::Fixed {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
        frame_offsets: Box::new([]),
        borrow_offsets: Box::default(),
    }
}
