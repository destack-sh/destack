use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::runtime::Runtime;

const ELEMENT_COUNT: u64 = 256;
const ITERATIONS: i32 = 1_000;

const PROGRAM: &str = r#"
function f0 {
    constant.int32 r1, 0
    constant.int32 r2, 1
    constant.int32 r3, 2
    constant.uint64 r4, 0
    tensor.splat r5, r2, a0
    tensor.splat r6, r3, a1
    tensor.splat r7, r1, a2

b0:
    branch.lt.int32 r1, r0 => b1 | b2

b1:
    tensor.element r7, [(r5, l0), (r6, l0)], add.int, a3
    add.int32 r1, r1, r2
    jump b0

b2:
    tensor.extract r8, (r7, l0), [r4]
    return r8
}
"#;

/// Benchmark allocation and elementwise direct CPU tensor execution.
pub(crate) fn bench_tensor(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vm_tensor");
    group.throughput(Throughput::Elements(ITERATIONS as u64 * ELEMENT_COUNT));
    group.bench_function("add.int32", |bencher| {
        bencher.iter_batched(
            || Runtime::tensor(PROGRAM, &[ELEMENT_COUNT]),
            |mut runtime| black_box(runtime.run(ITERATIONS)),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}
