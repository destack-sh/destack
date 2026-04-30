use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

use crate::common::{
    FORK_MATERIALIZED_PAGES, PAGE_BYTES, SPACE_BYTES, materialized_space, reserve_space,
};

/// Benchmark forkable address-space operations.
pub(crate) fn bench_address_space(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_address_space");
    group.throughput(Throughput::Bytes(SPACE_BYTES as u64));

    // reserve virtual memory without touching pages
    group.bench_function("reserve", |bencher| {
        bencher.iter(|| black_box(reserve_space()));
    });

    // materialize one page through the address-space write path
    group.bench_function("write_first_page", |bencher| {
        let page = vec![0xCD; PAGE_BYTES];

        bencher.iter_batched(
            reserve_space,
            |space| {
                space
                    .write(0, black_box(&page))
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
                    || materialized_space(*page_count),
                    |space| black_box(space.fork().expect("address space fork should succeed")),
                    BatchSize::SmallInput,
                );
            },
        );
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
                        let parent = materialized_space(*page_count);

                        parent.fork().expect("address space fork should succeed")
                    },
                    |child| {
                        child
                            .write(0, black_box(&page))
                            .expect("forked address space write should succeed")
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}
