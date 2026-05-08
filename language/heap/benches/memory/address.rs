use std::hint::black_box;
use std::ptr::write_volatile;
use std::time::{Duration, Instant};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

use crate::config::{
    FORK_ANCESTOR_COUNTS, FORK_DIRTY_PAGE_COUNTS, FORK_LARGE_ACTIVE_BYTES, FORK_LARGE_ANCESTORS,
    FORK_LARGE_DIRTY_BYTES, FORK_LARGE_SPACE_BYTES, FORK_MATERIALIZED_PAGES, PAGE_BYTES,
    SPACE_BYTES,
};
use crate::space::{AddressSpaceShape, ForkLineage};

/// Benchmark forkable address-space operations.
pub(crate) fn bench_address_space(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_address_space");
    group.throughput(Throughput::Bytes(SPACE_BYTES as u64));

    // reserve virtual memory without touching pages
    group.bench_function("reserve", |bencher| {
        bencher.iter(|| black_box(AddressSpaceShape::reserved().reserve()));
    });

    // materialize one page through the address-space write path
    group.bench_function("write_first_page", |bencher| {
        let page = vec![0xCD; PAGE_BYTES];

        bencher.iter_batched(
            || AddressSpaceShape::reserved().reserve(),
            |space| {
                space
                    .copy_bytes(0, black_box(&page))
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
                        black_box(
                            space
                                .fork_lazy()
                                .expect("address space fork should succeed"),
                        )
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

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
    for space_bytes in FORK_LARGE_SPACE_BYTES {
        for active_bytes in FORK_LARGE_ACTIVE_BYTES {
            for ancestor_count in FORK_LARGE_ANCESTORS {
                for dirty_bytes in large_dirty_bytes(*active_bytes) {
                    let name = byte_lineage_name(
                        *space_bytes,
                        *active_bytes,
                        *ancestor_count,
                        dirty_bytes,
                    );

                    group.bench_with_input(
                        BenchmarkId::new("fork_lineage_bytes", name),
                        &(*space_bytes, *active_bytes, *ancestor_count, dirty_bytes),
                        |bencher, &(space_bytes, active_bytes, ancestor_count, dirty_bytes)| {
                            bencher.iter_batched_ref(
                                || {
                                    ForkLineage::with_bytes(
                                        space_bytes,
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

    // measure the first bulk child copy after one fork
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork_copy_first_page", page_count),
            page_count,
            |bencher, page_count| {
                let page = vec![0xEF; PAGE_BYTES];

                bencher.iter_batched(
                    || {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);

                        shape.fork_lazy_pair()
                    },
                    |(parent, child)| {
                        child
                            .copy_bytes(0, black_box(&page))
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
                    || {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);

                        shape.fork_lazy_word()
                    },
                    |(parent, child, address)| {
                        // write through the exposed pointer to exercise the fault path
                        unsafe {
                            write_volatile(address, black_box(0xEFEF_EFEF_EFEF_EFEF));
                        }

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
                            let shape = AddressSpaceShape::materialized_pages(page_count);

                            shape.fork_lazy_pages(store_count)
                        },
                        |(parent, child, base_address)| {
                            // write one word per page to measure first-write fault count
                            for page_index in 0..store_count {
                                let byte_offset = page_index * PAGE_BYTES;
                                let address = unsafe { base_address.byte_add(byte_offset) };

                                unsafe {
                                    write_volatile(
                                        address,
                                        black_box(0xEFEF_EFEF_EFEF_EFEF ^ page_index),
                                    );
                                }
                            }

                            black_box(parent);
                            black_box(child);
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    // isolate fork, drop, fault, and steady-store phases
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork_phase/fork_eager", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_batched(
                    || AddressSpaceShape::materialized_pages(*page_count).materialize(),
                    |space| {
                        black_box(space.fork_eager(..).expect("eager fork should succeed"));
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
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);
                        let (parent, child) = shape.fork_lazy_pair();
                        let start = Instant::now();

                        drop(black_box(child));
                        elapsed += start.elapsed();
                        black_box(parent);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/first_fault", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);
                        let (parent, child, address) = shape.fork_lazy_word();
                        let start = Instant::now();

                        // write through the exposed pointer to time only the fault path
                        unsafe {
                            write_volatile(address, black_box(0xEFEF_EFEF_EFEF_EFEF));
                        }

                        elapsed += start.elapsed();
                        black_box(parent);
                        black_box(child);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/steady_store", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);
                        let (parent, child, address) = shape.fork_lazy_word();

                        // dirty the page before timing the second store
                        unsafe {
                            write_volatile(address, black_box(0xAAAA_AAAA_AAAA_AAAA));
                        }
                        let start = Instant::now();

                        // write again after the page is writable
                        unsafe {
                            write_volatile(address, black_box(0xBBBB_BBBB_BBBB_BBBB));
                        }

                        elapsed += start.elapsed();
                        black_box(parent);
                        black_box(child);
                    }

                    elapsed
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fork_phase/eager_first_store", page_count),
            page_count,
            |bencher, page_count| {
                bencher.iter_custom(|iterations| {
                    let mut elapsed = Duration::ZERO;

                    for _ in 0..iterations {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);
                        let (parent, child, address) = shape.fork_eager_word();
                        let start = Instant::now();

                        // eager fork should already make this page writable
                        unsafe {
                            write_volatile(address, black_box(0xDDDD_DDDD_DDDD_DDDD));
                        }

                        elapsed += start.elapsed();
                        black_box(parent);
                        black_box(child);
                    }

                    elapsed
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
                        let mut elapsed = Duration::ZERO;

                        for _ in 0..iterations {
                            let shape = AddressSpaceShape::materialized_pages(page_count);
                            let (parent, child, base_address) = shape.fork_lazy_pages(store_count);
                            let start = Instant::now();

                            // write one word per page to time only first-write faults
                            for page_index in 0..store_count {
                                let byte_offset = page_index * PAGE_BYTES;
                                let address = unsafe { base_address.byte_add(byte_offset) };

                                unsafe {
                                    write_volatile(
                                        address,
                                        black_box(0xCFCF_CFCF_CFCF_CFCF ^ page_index),
                                    );
                                }
                            }

                            elapsed += start.elapsed();
                            black_box(parent);
                            black_box(child);
                        }

                        elapsed
                    });
                },
            );
        }
    }

    group.finish();
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
    space_bytes: usize,
    active_bytes: usize,
    ancestor_count: usize,
    dirty_bytes: usize,
) -> String {
    format!(
        "space={}/active={}/ancestors={ancestor_count}/dirty={}",
        mib(space_bytes),
        mib(active_bytes),
        mib(dirty_bytes)
    )
}

/// Return a MiB label.
fn mib(bytes: usize) -> String {
    let mib = bytes / (1024 * 1024);

    format!("{mib}MiB")
}
