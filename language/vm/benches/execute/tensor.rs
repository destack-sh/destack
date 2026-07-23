use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::runtime::Runtime;

const ELEMENT_COUNT: u64 = 256;
const ITERATIONS: i32 = 1_000;

const PROGRAM: &str = r#"
type Vector

export function add(r0: int32): int32 {
    r1: int32 = 0
    r2: int32 = 1
    r3: int32 = 2
    r4: uint64 = 0
    r5: tensor<int32, Vector, space(local)> = tensor.splat r2
    r6: tensor<int32, Vector, space(local)> = tensor.splat r3
    r7: tensor<int32, Vector, space(local)> = tensor.splat r1

l0:
    branch.lt.int32 r1, r0 => l1, l2

l1:
    r7: tensor<int32, Vector, space(local)> = int.add r5, r6
    r1: int32 = int.add r1, r2
    jump l0

l2:
    r8: int32 = tensor.extract r7, [r4]
    return r8
}
"#;

/// Benchmark allocation and elementwise direct CPU tensor execution.
pub(crate) fn bench_tensor(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vm_tensor");
    group.throughput(Throughput::Elements(ITERATIONS as u64 * ELEMENT_COUNT));
    group.bench_function("int32.add", |bencher| {
        bencher.iter_batched(
            || Runtime::tensor(PROGRAM, &[ELEMENT_COUNT]),
            |mut runtime| black_box(runtime.run(ITERATIONS)),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}
