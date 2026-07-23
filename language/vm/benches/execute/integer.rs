use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::runtime::Runtime;

const ITERATIONS: i32 = 10_000;

const PROGRAM: &str = r#"
export function add(r0: int32): int32 {
    r1: int32 = 0
    r2: int32 = 1
    r3: int32 = 0

l0:
    branch.lt.int32 r1, r0 => l1, l2

l1:
    r3: int32 = int.add r3, r1
    r1: int32 = int.add r1, r2
    jump l0

l2:
    return r3
}
"#;

/// Benchmark integer dispatch inside one bytecode invocation.
pub(crate) fn bench_integer(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vm_integer");
    group.throughput(Throughput::Elements(ITERATIONS as u64));
    group.bench_function("int32.add", |bencher| {
        bencher.iter_batched(
            || Runtime::parse(PROGRAM),
            |mut runtime| black_box(runtime.run(ITERATIONS)),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}
