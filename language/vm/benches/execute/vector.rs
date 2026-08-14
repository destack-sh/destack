use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};

use crate::runtime::Runtime;

const ITERATIONS: i32 = 10_000;

const PROGRAM: &str = r#"
function f0 {
    constant.int32 r1, 0
    constant.int32 r2, 1
    vector.splat.int32x4 r3:r4, r1
    vector.splat.int32x4 r5:r6, r2

b0:
    branch.lt.int32 r1, r0 => b1 | b2

b1:
    vector.add.int32x4 r7:r8, r3:r4, r5:r6
    move r3:r4, r7:r8
    add.int32 r1, r1, r2
    jump b0

b2:
    constant.uint32 r9, 0
    vector.extract.int32x4 r10, r3:r4, r9
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
