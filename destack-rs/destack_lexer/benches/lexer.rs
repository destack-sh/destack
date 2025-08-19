use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

const BENCHMARK_CASES: &[(&str, &str)] = &[
    (
        "rigid",
        include_str!("../../../destack-ds/destack_examples/destack/rigid.ds"),
    ),
    (
        "view",
        include_str!("../../../destack-ds/destack_examples/destack/view.ds"),
    ),
    (
        "tetris",
        include_str!("../../../destack-ds/destack_examples/tetris/tetris.ds"),
    ),
];

fn bench_tokenize(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");
    for (name, input) in BENCHMARK_CASES {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("tokenize", name), input, |b, &src| {
            b.iter(|| {
                // iterate tokens without allocating
                // NOTE: count to avoid being optimized away
                let mut n = 0u32;
                for _tok in destack_lexer::tokenize(src) {
                    n += 1;
                }
                black_box(n);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_tokenize);
criterion_main!(benches);
