mod program;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_vm::memory::Value;
use pprof::criterion::{Output, PProfProfiler};
use program::Program;
use std::env;

/// Run a benchmark program at various input sizes.
fn bench_program(c: &mut Criterion, group_name: &str, program: &Program, sizes: &[i64]) {
    let mut group = c.benchmark_group(group_name);
    let mut interp = program.interpreter();
    let base_args = (program.default_args)();

    for &n in sizes {
        // first arg is always the iteration count
        let mut args = base_args.clone();
        if !args.is_empty() {
            args[0] = Value::int64(n);
        }

        // get actual instruction count from a test run
        let ops = program.actual_instruction_count(&args);
        group.throughput(Throughput::Elements(ops));

        // run the benchmark
        group.bench_with_input(BenchmarkId::new(program.name, n), &n, |b, _| {
            b.iter(|| {
                let result = interp
                    .run_function_by_name(program.entry, &args)
                    .expect("execution failed");
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Dispatch benchmarks: loops, branches, switches.
fn bench_dispatch(c: &mut Criterion) {
    let sizes = &[1_000, 10_000, 100_000];

    for program in program::dispatch::ALL {
        bench_program(c, "dispatch", program, sizes);
    }
}

/// Arithmetic benchmarks: fibonacci, primes, integer and float ops.
fn bench_arithmetic(c: &mut Criterion) {
    // fib_recursive gets special small sizes due to exponential growth
    bench_program(
        c,
        "arithmetic",
        &program::arithmetic::FIB_RECURSIVE,
        &[20, 25, 30],
    );

    // other arithmetic benchmarks use standard sizes
    let sizes = &[1_000, 10_000, 100_000];
    for program in program::arithmetic::ALL.iter().skip(1) {
        bench_program(c, "arithmetic", program, sizes);
    }
}

/// Function call benchmarks: shallow, deep, many args, recursive.
fn bench_calls(c: &mut Criterion) {
    let sizes = &[100, 1_000, 10_000];

    for program in program::calls::ALL {
        bench_program(c, "calls", program, sizes);
    }
}

/// Memory benchmarks: allocation, load/store, linked list.
fn bench_memory(c: &mut Criterion) {
    let sizes = &[100, 1_000, 10_000];

    for program in program::memory::ALL {
        let mut interp = program.interpreter();
        let mut group = c.benchmark_group("memory");

        for &n in sizes {
            let args = [Value::int64(n)];

            // get actual instruction count
            let ops = program.actual_instruction_count(&args);
            group.throughput(Throughput::Elements(ops));

            // benchmark with gc before each run
            group.bench_with_input(BenchmarkId::new(program.name, n), &n, |b, &n| {
                b.iter(|| {
                    interp.collect_garbage();
                    let result = interp
                        .run_function_by_name(program.entry, &[Value::int64(n)])
                        .expect("execution failed");
                    black_box(result)
                });
            });
        }

        group.finish();
    }
}

/// Intrinsics benchmarks: bit ops, float math, checked arithmetic.
fn bench_intrinsics(c: &mut Criterion) {
    let sizes = &[1_000, 10_000, 100_000];

    for program in program::intrinsics::ALL {
        bench_program(c, "intrinsics", program, sizes);
    }
}

/// Create criterion configuration with pprof profiler.
fn profiler() -> Criterion {
    let hz = env::var("DESTACK_PPROF_HZ")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    Criterion::default().with_profiler(PProfProfiler::new(hz, Output::Flamegraph(None)))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_dispatch, bench_arithmetic, bench_calls, bench_memory, bench_intrinsics
}
criterion_main!(benches);

#[test]
fn validate_all_programs() {
    program::validate_all();
}

#[test]
fn print_stats() {
    program::print_stats();
}
