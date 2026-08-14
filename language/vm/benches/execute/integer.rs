use std::hint::black_box;

use criterion::{Criterion, Throughput};

use crate::runtime::Runtime;

const ITERATIONS: i32 = 100_000;

const PROGRAM: &str = r#"
function f0 {
    constant.int32 r1, 0
    constant.int32 r2, 1
    constant.int32 r3, 0

b0:
    branch.lt.int32 r1, r0 => b1 | b2

b1:
    add.int32 r3, r3, r1
    add.int32 r1, r1, r2
    jump b0

b2:
    return r3
}
"#;

/// Benchmark integer dispatch inside one bytecode invocation.
pub(crate) fn bench_integer(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vm_integer");
    group.throughput(Throughput::Elements(ITERATIONS as u64));
    group.bench_function("add.int32", |bencher| {
        let mut runtime = Runtime::parse(PROGRAM);

        bencher.iter(|| black_box(runtime.run(ITERATIONS)));
    });
    group.finish();
}
