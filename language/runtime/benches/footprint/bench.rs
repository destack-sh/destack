mod measure;
mod report;
mod scenario;

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use scenario::{RuntimeScenario, VmScenario};

#[global_allocator]
static ALLOCATOR: measure::CountingAllocator = measure::CountingAllocator;

/// Benchmark runtime footprint scenarios.
fn bench_footprint(criterion: &mut Criterion) {
    report::print_once();

    let runtime = RuntimeScenario::new();
    let vm = VmScenario::new();
    let mut group = criterion.benchmark_group("runtime/footprint");

    group.bench_function("world.new", |bencher| {
        bencher.iter(|| black_box(runtime.world()))
    });

    group.bench_function("runtime.spawn.empty.vm", |bencher| {
        bencher.iter_batched(
            || (runtime.world(), runtime.engine()),
            |(mut world, engine)| black_box(runtime.spawn_runtime(&mut world, engine)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("worker.spawn.empty.vm", |bencher| {
        bencher.iter_batched(
            || {
                let (world, runtime_id) = runtime.world_with_runtime();
                let engine = runtime.engine();

                (world, runtime_id, engine)
            },
            |(mut world, runtime_id, engine)| {
                black_box(runtime.spawn_worker(&mut world, runtime_id, engine))
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("launch.empty", |bencher| {
        bencher.iter(|| black_box(runtime.launch()))
    });

    group.bench_function("vm.isolate.build", |bencher| {
        bencher.iter(|| black_box(vm.isolate()))
    });

    group.bench_function("vm.machine.new", |bencher| {
        bencher.iter(|| black_box(vm.machine()))
    });

    group.bench_function("vm.continuation.yield", |bencher| {
        bencher.iter_batched(
            || vm.machine(),
            |mut machine| black_box(machine.yield_once()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("vm.continuation.image", |bencher| {
        bencher.iter_batched(
            || {
                let mut machine = vm.machine();
                let continuation = machine.yield_once();

                (machine, continuation)
            },
            |(machine, continuation)| black_box(machine.continuation_image(&continuation)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("vm.continuation.clone", |bencher| {
        bencher.iter_batched(
            || {
                let mut machine = vm.machine();

                machine.yield_once()
            },
            |continuation| black_box(continuation.fork()),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_footprint);
criterion_main!(benches);
