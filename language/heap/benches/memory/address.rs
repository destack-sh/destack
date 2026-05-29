use std::hint::black_box;
use std::ptr::write_volatile;
use std::time::{Duration, Instant};

use criterion::measurement::WallTime;
use criterion::{BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput};

use crate::config::{
    FORK_ANCESTOR_COUNTS, FORK_DIRTY_PAGE_COUNTS, FORK_LARGE_ACTIVE_BYTES, FORK_LARGE_ANCESTORS,
    FORK_LARGE_DIRTY_BYTES, FORK_LARGE_SPACE_SIZE_BYTES, FORK_MATERIALIZED_PAGES, PAGE_SIZE_BYTES,
    SPACE_SIZE_BYTES,
};
use crate::space::{AddressSpaceShape, ForkLineage};

/// Benchmark forkable address-space operations.
pub(crate) fn bench_address_space(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_address_space");
    group.throughput(Throughput::Bytes(SPACE_SIZE_BYTES as u64));

    bench_reservation(&mut group);
    bench_lineage_forks(&mut group);
    bench_fork_writes(&mut group);
    bench_fork_phases(&mut group);

    group.finish();
}

/// Register basic address-space reservation and materialization benchmarks.
fn bench_reservation(group: &mut BenchmarkGroup<'_, WallTime>) {
    // reserve virtual memory without touching pages
    group.bench_function("reserve", |bencher| {
        bencher.iter(|| black_box(AddressSpaceShape::reserved().reserve()));
    });

    // materialize one page through the address-space write path
    group.bench_function("write_first_page", |bencher| {
        let page = vec![0xCD; PAGE_SIZE_BYTES];

        bencher.iter_batched(
            || AddressSpaceShape::reserved().reserve(),
            |space| {
                space
                    .write_bytes(0, black_box(&page))
                    .expect("address space write should succeed")
            },
            BatchSize::SmallInput,
        );
    });

    // fork materialized address spaces across representative sizes
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_batched(
                    || AddressSpaceShape::materialized_pages(*page_count).materialize(),
                    |space| {
                        let child = space
                            .fork_lazy()
                            .expect("address space fork should succeed");

                        black_box(child)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
}

/// Register nested lineage fork benchmarks.
fn bench_lineage_forks(group: &mut BenchmarkGroup<'_, WallTime>) {
    // fork a live lineage across nesting and dirty-page counts
    for page_count in FORK_MATERIALIZED_PAGES {
        for ancestor_count in FORK_ANCESTOR_COUNTS {
            for dirty_page_count in dirty_page_counts(*page_count) {
                let name = page_lineage_name(*page_count, *ancestor_count, dirty_page_count);

                group.bench_with_input(
                    BenchmarkId::new("fork_lineage_pages", name),
                    &(*page_count, *ancestor_count, dirty_page_count),
                    |bencher, &(page_count, ancestor_count, dirty_page_count)| {
                        bencher.iter_batched_ref(
                            || {
                                ForkLineage::with_pages(
                                    page_count,
                                    ancestor_count,
                                    dirty_page_count,
                                )
                            },
                            |lineage| {
                                black_box(lineage.fork_leaf());
                            },
                            BatchSize::SmallInput,
                        );
                    },
                );
            }
        }
    }

    // fork large live heaps across reserved, active, dirty, and nested bytes
    for space_size_bytes in FORK_LARGE_SPACE_SIZE_BYTES {
        for active_bytes in FORK_LARGE_ACTIVE_BYTES {
            for ancestor_count in FORK_LARGE_ANCESTORS {
                for dirty_bytes in large_dirty_bytes(*active_bytes) {
                    let name = byte_lineage_name(
                        *space_size_bytes,
                        *active_bytes,
                        *ancestor_count,
                        dirty_bytes,
                    );

                    group.bench_with_input(
                        BenchmarkId::new("fork_lineage_bytes", name),
                        &(*space_size_bytes, *active_bytes, *ancestor_count, dirty_bytes),
                        |bencher, &(space_size_bytes, active_bytes, ancestor_count, dirty_bytes)| {
                            bencher.iter_batched_ref(
                                || {
                                    ForkLineage::with_bytes(
                                        space_size_bytes,
                                        active_bytes,
                                        ancestor_count,
                                        dirty_bytes,
                                    )
                                },
                                |lineage| {
                                    black_box(lineage.fork_leaf());
                                },
                                BatchSize::SmallInput,
                            );
                        },
                    );
                }
            }
        }
    }
}

/// Register first-write fork benchmarks.
fn bench_fork_writes(group: &mut BenchmarkGroup<'_, WallTime>) {
    // measure the first bulk child copy after one fork
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork_copy_first_page", page_count),
            page_count,
            |bencher, page_count| {
                let page = vec![0xEF; PAGE_SIZE_BYTES];

                bencher.iter_batched(
                    || AddressSpaceShape::materialized_pages(*page_count).fork_lazy_pair(),
                    |(parent, child)| {
                        child
                            .write_bytes(0, black_box(&page))
                            .expect("forked address space write should succeed");

                        black_box(parent);
                        black_box(child);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    // measure the first scalar child store after one fork
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork_store_first_page", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_batched(
                    || AddressSpaceShape::materialized_pages(*page_count).fork_lazy_word(),
                    |(parent, child, address)| {
                        write_word(address, black_box(0xEFEF_EFEF_EFEF_EFEF));

                        black_box(parent);
                        black_box(child);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    // measure distinct first scalar stores after one fork
    for page_count in FORK_MATERIALIZED_PAGES {
        for store_count in dirty_page_counts(*page_count) {
            if store_count == 0 {
                continue;
            }

            let name = store_page_name(*page_count, store_count);
            group.bench_with_input(
                BenchmarkId::new("fork_store_pages", name),
                &(*page_count, store_count),
                |bencher, &(page_count, store_count)| {
                    bencher.iter_batched(
                        || {
                            AddressSpaceShape::materialized_pages(page_count)
                                .fork_lazy_pages(store_count)
                        },
                        |(parent, child, base_address)| {
                            write_page_words(base_address, store_count, 0xEFEF_EFEF_EFEF_EFEF);

                            black_box(parent);
                            black_box(child);
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }
}

/// Register fork phase isolation benchmarks.
fn bench_fork_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork_phase/fork_eager", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_batched(
                    || AddressSpaceShape::materialized_pages(*page_count).materialize(),
                    |space| {
                        let child = space
                            .fork_eager(0..space.byte_len())
                            .expect("eager fork should succeed");

                        black_box(child);
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/drop_child", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    measure_address_phase(
                        iterations,
                        || AddressSpaceShape::materialized_pages(*page_count).fork_lazy_pair(),
                        |(parent, child)| {
                            drop(black_box(child));

                            parent
                        },
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/first_fault", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    measure_address_phase(
                        iterations,
                        || AddressSpaceShape::materialized_pages(*page_count).fork_lazy_word(),
                        |(parent, child, address)| {
                            write_word(address, black_box(0xEFEF_EFEF_EFEF_EFEF));

                            (parent, child)
                        },
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/steady_store", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    measure_address_phase(
                        iterations,
                        || {
                            let fork =
                                AddressSpaceShape::materialized_pages(*page_count).fork_lazy_word();

                            write_word(fork.2, black_box(0xAAAA_AAAA_AAAA_AAAA));

                            fork
                        },
                        |(parent, child, address)| {
                            write_word(address, black_box(0xBBBB_BBBB_BBBB_BBBB));

                            (parent, child)
                        },
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/eager_first_store", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    measure_address_phase(
                        iterations,
                        || AddressSpaceShape::materialized_pages(*page_count).fork_eager_word(),
                        |(parent, child, address)| {
                            write_word(address, black_box(0xDDDD_DDDD_DDDD_DDDD));

                            (parent, child)
                        },
                    )
                });
            },
        );

        for store_count in dirty_page_counts(*page_count) {
            if store_count == 0 {
                continue;
            }

            let name = store_page_name(*page_count, store_count);
            group.bench_with_input(
                BenchmarkId::new("fork_phase/fault_pages", name),
                &(*page_count, store_count),
                |bencher, &(page_count, store_count)| {
                    bencher.iter_custom(|iterations| {
                        measure_address_phase(
                            iterations,
                            || {
                                AddressSpaceShape::materialized_pages(page_count)
                                    .fork_lazy_pages(store_count)
                            },
                            |(parent, child, base_address)| {
                                write_page_words(base_address, store_count, 0xCFCF_CFCF_CFCF_CFCF);

                                (parent, child)
                            },
                        )
                    });
                },
            );
        }
    }
}

/// Measure one address-space phase with setup outside the timed region.
fn measure_address_phase<T, K>(
    iterations: u64,
    mut prepare: impl FnMut() -> T,
    mut measure: impl FnMut(T) -> K,
) -> Duration {
    let mut elapsed = Duration::ZERO;

    // rebuild the address-space shape for each criterion iteration
    for _ in 0..iterations {
        let input = prepare();
        let start = Instant::now();

        let keepalive = measure(input);

        elapsed += start.elapsed();
        black_box(keepalive);
    }

    elapsed
}

/// Store one volatile word through an exposed benchmark address.
#[inline(always)]
fn write_word(address: *mut usize, value: usize) {
    // address-space fixtures expose valid writable word addresses
    unsafe {
        write_volatile(address, value);
    }
}

/// Store one volatile word on each benchmark page.
#[inline(always)]
fn write_page_words(base_address: *mut usize, page_count: usize, seed: usize) {
    // write one word per page to time first-write faults
    for page_index in 0..page_count {
        let value = seed ^ page_index;

        write_page_word(base_address, page_index, black_box(value));
    }
}

/// Store one volatile word on one benchmark page.
#[inline(always)]
fn write_page_word(base_address: *mut usize, page_index: usize, value: usize) {
    let byte_offset = page_index * PAGE_SIZE_BYTES;

    // address-space fixtures expose page-aligned word ranges
    unsafe {
        let address = base_address.byte_add(byte_offset);

        write_volatile(address, value);
    }
}

/// Return the dirty page counts that fit inside one materialized page count.
fn dirty_page_counts(page_count: usize) -> impl Iterator<Item = usize> {
    FORK_DIRTY_PAGE_COUNTS
        .iter()
        .copied()
        .filter(move |dirty_page_count| *dirty_page_count <= page_count)
}

/// Return the page-lineage benchmark label.
fn page_lineage_name(page_count: usize, ancestor_count: usize, dirty_page_count: usize) -> String {
    format!("pages={page_count}/ancestors={ancestor_count}/dirty={dirty_page_count}")
}

/// Return the store-page benchmark label.
fn store_page_name(page_count: usize, store_count: usize) -> String {
    format!("pages={page_count}/stores={store_count}")
}

/// Return dirty byte counts that fit inside one active byte count.
fn large_dirty_bytes(active_bytes: usize) -> impl Iterator<Item = usize> {
    FORK_LARGE_DIRTY_BYTES
        .iter()
        .copied()
        .filter(move |dirty_bytes| *dirty_bytes <= active_bytes)
}

/// Return the byte-lineage benchmark label.
fn byte_lineage_name(
    space_size_bytes: usize,
    active_bytes: usize,
    ancestor_count: usize,
    dirty_bytes: usize,
) -> String {
    format!(
        "space={}/active={}/ancestors={ancestor_count}/dirty={}",
        mib(space_size_bytes),
        mib(active_bytes),
        mib(dirty_bytes)
    )
}

/// Return a MiB label.
fn mib(bytes: usize) -> String {
    let mib = bytes / (1024 * 1024);

    format!("{mib}MiB")
}
