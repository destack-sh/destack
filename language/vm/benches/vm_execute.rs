mod program;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use destack_vm::memory::Value;
use pprof::criterion::{Output, PProfProfiler};
use program::Program;
use std::env;

/// Build input sizes for a benchmark program.
fn benchmark_sizes(program: &Program) -> Vec<i64> {
    // prefer the calibratable axis if available
    let axis = program
        .scales
        .iter()
        .find(|axis| axis.calibrate)
        .or_else(|| program.scales.first());

    // fall back to a single size when no axes exist
    let Some(axis) = axis else {
        return vec![1];
    };

    // collect unique sizes
    let mut sizes = vec![axis.quick, axis.standard, axis.stress];
    sizes.retain(|value| *value > 0);
    sizes.sort_unstable();
    sizes.dedup();

    // return size list
    sizes
}

/// Run a benchmark program at various input sizes.
fn bench_program(c: &mut Criterion, group_name: &str, program: &Program) {
    let mut group = c.benchmark_group(group_name);
    let mut isolate = program.isolate();
    let entry_id = program.entry_id(&isolate);
    let base_args = program.args_for_profile(&isolate, program::BenchProfileKind::Quick);
    let sizes = benchmark_sizes(program);
    let scale_axis = program
        .scales
        .iter()
        .find(|axis| axis.calibrate)
        .map(|axis| axis.arg_index);

    for &n in &sizes {
        // apply scale axis for the program
        let mut args = base_args.clone();
        if let Some(axis) = scale_axis
            && axis < args.len()
        {
            args[axis] = Value::int64(n);
        }

        // get actual instruction count from a test run
        let ops = program.actual_instruction_count(&args);
        group.throughput(Throughput::Elements(ops));

        // run the benchmark
        group.bench_with_input(BenchmarkId::new(program.name, n), &n, |b, _| {
            b.iter(|| {
                let result = program.run_or_panic(&mut isolate, entry_id, &args);
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Dispatch benchmarks: loops, branches, switches.
fn bench_dispatch(c: &mut Criterion) {
    for program in program::dispatch::ALL {
        bench_program(c, "dispatch", program);
    }
}

/// Arithmetic benchmarks: fibonacci, primes, integer and float ops.
fn bench_arithmetic(c: &mut Criterion) {
    for program in program::arithmetic::ALL {
        bench_program(c, "arithmetic", program);
    }
}

/// Function call benchmarks: shallow, deep, many args, recursive.
fn bench_calls(c: &mut Criterion) {
    for program in program::calls::ALL {
        bench_program(c, "calls", program);
    }
}

/// Memory benchmarks: allocation, load/store, linked list.
fn bench_memory(c: &mut Criterion) {
    for program in program::memory::ALL {
        let mut isolate = program.isolate();
        let entry_id = program.entry_id(&isolate);
        let base_args = program.args_for_profile(&isolate, program::BenchProfileKind::Quick);
        let sizes = benchmark_sizes(program);
        let scale_axis = program
            .scales
            .iter()
            .find(|axis| axis.calibrate)
            .map(|axis| axis.arg_index);
        let mut group = c.benchmark_group("memory");

        for &n in &sizes {
            let mut args = base_args.clone();
            if let Some(axis) = scale_axis
                && axis < args.len()
            {
                args[axis] = Value::int64(n);
            }

            // get actual instruction count
            let ops = program.actual_instruction_count(&args);
            group.throughput(Throughput::Elements(ops));

            // benchmark with gc before each run
            group.bench_with_input(BenchmarkId::new(program.name, n), &n, |b, _| {
                b.iter(|| {
                    let _ = isolate.collect_garbage();
                    let result = program.run_or_panic(&mut isolate, entry_id, &args);
                    black_box(result)
                });
            });
        }

        group.finish();
    }
}

/// Intrinsics benchmarks: bit ops, float math, checked arithmetic.
fn bench_intrinsics(c: &mut Criterion) {
    for program in program::intrinsics::ALL {
        bench_program(c, "intrinsics", program);
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
    let options = program::BenchOptions::new(
        None,
        None,
        program::BenchOutputFormat::Table,
        program::BenchProfileKind::Quick.defaults(),
        false,
        false,
        None,
        false,
        false,
    );
    program::print_stats(&options);
}
