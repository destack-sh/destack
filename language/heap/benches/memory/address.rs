use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};

use crate::common::{
    FORK_MATERIALIZED_PAGES, MATRIX_ALLOCATIONS, PAGE_BYTES, PRETOUCH_BYTES, SMALL_BYTES,
    SPACE_BYTES, materialized_space, reserve_space,
};

/// Benchmark forkable address-space operations.
pub(crate) fn bench_address_space(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("heap_address_space");
    group.throughput(Throughput::Bytes(SPACE_BYTES as u64));

    group.bench_function("reserve", |bencher| {
        bencher.iter(|| black_box(reserve_space()));
    });

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

    group.throughput(Throughput::Bytes(PRETOUCH_BYTES as u64));

    group.bench_function("write_pretouched_32", |bencher| {
        let payload = [0xAB; SMALL_BYTES];

        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;

            for _ in 0..iterations {
                let space = materialized_space(PRETOUCH_BYTES.div_ceil(PAGE_BYTES));
                let address = space
                    .address(0, PRETOUCH_BYTES)
                    .expect("address space address should resolve");

                let start = Instant::now();

                for index in 0..MATRIX_ALLOCATIONS {
                    let offset = index * SMALL_BYTES;

                    // raw address copy lower bound
                    unsafe {
                        address
                            .add(offset)
                            .cast::<[u8; SMALL_BYTES]>()
                            .write(payload);
                    }
                }

                elapsed += start.elapsed();
                black_box(space);
            }

            elapsed
        });
    });

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
