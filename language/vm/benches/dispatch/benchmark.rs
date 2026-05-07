use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{BatchSize, BenchmarkGroup, Criterion, Throughput};

use crate::program::Program;
use crate::runtime::Runtime;

/// The number of loop iterations each benchmarked VM call executes.
pub(crate) const ITERATIONS: i32 = 10_000;
/// The number of logical operations represented by one benchmark call.
const OPERATION_COUNT: u64 = ITERATIONS as u64;

/// Create one benchmark group with VM dispatch throughput.
pub(crate) fn group<'a>(criterion: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    let mut group = criterion.benchmark_group(name);
    group.throughput(Throughput::Elements(OPERATION_COUNT));

    group
}

/// Benchmark one MIR program entry.
pub(crate) fn program(group: &mut BenchmarkGroup<'_, WallTime>, program: Program) {
    let source = program.read();

    group.bench_function(program.name, |bencher| {
        bencher.iter_batched(
            || Runtime::new(&source, program.entry),
            |mut runtime| black_box(runtime.run(ITERATIONS)),
            BatchSize::SmallInput,
        );
    });
}
