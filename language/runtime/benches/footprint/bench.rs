mod measure;
mod report;
mod setup;

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use setup::{RuntimeSetup, VmSetup};

#[global_allocator]
static ALLOCATOR: measure::CountingAllocator = measure::CountingAllocator;

/// Benchmark runtime footprint setups.
fn bench_footprint(criterion: &mut Criterion) {
    report::print_once();

    let runtime = RuntimeSetup::new();
    let vm = VmSetup::new();
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

                (world, runtime_id)
            },
            |(mut world, runtime_id)| {
                runtime.spawn_worker(&mut world, runtime_id);

                black_box(())
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("execution.empty", |bencher| {
        bencher.iter(|| {
            runtime.execute();

            black_box(())
        })
    });

    group.bench_function("vm.machine.build", |bencher| {
        bencher.iter(|| black_box(vm.build_machine()))
    });

    group.bench_function("vm.machine.new", |bencher| {
        bencher.iter(|| black_box(vm.machine()))
    });

    group.bench_function("vm.machine.run", |bencher| {
        bencher.iter_batched(
            || vm.machine(),
            |mut machine| black_box(machine.run()),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_footprint);
criterion_main!(benches);
