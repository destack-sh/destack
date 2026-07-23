use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::runtime::Runtime;

const ITERATIONS: i32 = 10_000;

const PROGRAM: &str = r#"
export function add(r0: int32): int32 {
    r1: int32 = 0
    r2: int32 = 1
    r3: vector<int32, 4> = vector.splat r1
    r5: vector<int32, 4> = vector.splat r2

l0:
    branch.lt.int32 r1, r0 => l1, l2

l1:
    r7: vector<int32, 4> = int.add r3, r5
    r3: vector<int32, 4> = move r7
    r1: int32 = int.add r1, r2
    jump l0

l2:
    r9: uint32 = 0
    r10: int32 = vector.extract r3, r9
    return r10
}
"#;

/// Benchmark packed vector execution inside one bytecode invocation.
pub(crate) fn bench_vector(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vm_vector");
    group.throughput(Throughput::Elements(ITERATIONS as u64 * 4));
    group.bench_function("int32x4.add", |bencher| {
        bencher.iter_batched(
            || Runtime::parse(PROGRAM),
            |mut runtime| black_box(runtime.run(ITERATIONS)),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}
