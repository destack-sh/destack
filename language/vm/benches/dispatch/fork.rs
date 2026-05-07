use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion};
use destack_engine::Value;

use crate::benchmark::{self, ITERATIONS};
use crate::program::Program;
use crate::runtime::Runtime;

/// Benchmark VM-built heap forking workloads.
pub(crate) fn bench_fork(criterion: &mut Criterion) {
    let mut group = benchmark::group(criterion, "vm_heap_fork");
    let program = Program {
        name: "heap_fork_mutate",
        entry: "buildForkGraph",
    };
    let source = program.read();

    group.bench_function(BenchmarkId::new("clean", ITERATIONS), |bencher| {
        bencher.iter_batched(
            || {
                let mut runtime = Runtime::new(&source, program.entry);
                let root = runtime.run(ITERATIONS);

                (runtime, root)
            },
            |(mut runtime, root)| {
                let fork = runtime.heap.fork().expect("benchmark heap should fork");

                black_box((fork, root))
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function(BenchmarkId::new("mutate_hot_cell", ITERATIONS), |bencher| {
        bencher.iter_batched(
            || {
                let mut runtime = Runtime::new(&source, program.entry);
                let root = runtime.run(ITERATIONS);
                let mutate = runtime.entry("mutateHotCell");

                (runtime, root, mutate)
            },
            |(mut runtime, root, mutate)| {
                runtime.heap = runtime.heap.fork().expect("benchmark heap should fork");
                let arguments = [root, Value::int32(ITERATIONS)];
                let output = runtime.run_with_arguments(mutate, &arguments);

                black_box(output)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}
