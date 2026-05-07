use std::hint::black_box;

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
                    |space| black_box(space.fork().expect("address space fork should succeed")),
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

    // measure the first child write after one fork
    for page_count in FORK_MATERIALIZED_PAGES {
        group.bench_with_input(
            BenchmarkId::new("fork_write_first_page", page_count),
            page_count,
            |bencher, page_count| {
                let page = vec![0xEF; PAGE_BYTES];

                bencher.iter_batched(
                    || {
                        let shape = AddressSpaceShape::materialized_pages(*page_count);
                        let parent = shape.materialize();

                        parent.fork().expect("address space fork should succeed")
                    },
                    |child| {
                        child
                            .copy_bytes(0, black_box(&page))
                            .expect("forked address space write should succeed")
                    },
                    BatchSize::SmallInput,
                );
            },
        );
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
