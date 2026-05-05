use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs};

use criterion::measurement::WallTime;
use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use destack_engine::{EngineId, StaticSpace, Value};
use destack_heap::{
    Allocator, Heap, HeapLimits, HeapOptions, SharedAllocator, SharedGcWorker, SharedHeap,
    SharedHeapLimits,
};
use destack_mir as mir;
use destack_source::FileId;
use destack_vm::{Isolate, IsolateOptions};
use mir::parse::{ParseOptions, Parser};

/// The number of loop iterations each benchmarked VM call executes.
const ITERATIONS: i32 = 10_000;

/// The number of logical operations represented by one benchmark call.
const OPERATION_COUNT: u64 = ITERATIONS as u64;

/// Directory containing benchmark MIR programs.
const PROGRAM_DIRECTORY: &str = "benches/dispatch/program";

/// One MIR program measured by the dispatch benchmark.
#[derive(Debug, Clone, Copy)]
struct ProgramBench {
    /// The fixture stem and Criterion benchmark name.
    name: &'static str,
    /// The function called by the benchmark harness.
    entry: &'static str,
}

impl ProgramBench {
    /// Read the MIR program for this benchmark.
    fn read(self) -> String {
        // resolve the fixture relative to the vm crate
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(PROGRAM_DIRECTORY);
        path.push(format!("{}.mir", self.name));

        fs::read_to_string(&path).expect("benchmark MIR should be readable")
    }
}

/// Runtime state needed to call one benchmark entry.
struct Fixture {
    /// The isolate under measurement.
    isolate: Isolate,
    /// The worker static byte space.
    statics: StaticSpace,
    /// The worker heap.
    heap: Heap,
    /// The runtime shared heap.
    shared: SharedHeap,
    /// The worker-local shared allocator.
    shared_allocator: SharedAllocator,
    /// The shared collector worker.
    shared_gc: SharedGcWorker,
    /// The benchmark entry function.
    entry: mir::LocalNodeId<mir::Function>,
}

impl Fixture {
    /// Build one fixture from MIR text and entry name.
    fn new(program: &str, entry: &str) -> Self {
        // parse the benchmark program
        let (tree, strings) = Parser::parse(FileId::new(0), program, ParseOptions::default())
            .validate()
            .expect("benchmark MIR should parse");

        // build the isolate
        let mut isolate = Isolate::build_with_options(
            EngineId::new(1),
            tree,
            strings,
            IsolateOptions::unbounded(),
        )
        .expect("benchmark isolate should build");

        // build runtime memory
        let mut statics = StaticSpace::empty();
        let heap = heap();
        let shared = shared_heap();
        let shared_gc = shared.register_collector_worker();
        let shared_allocator = shared.allocator();

        // initialize program statics
        isolate
            .initialize(&heap, &shared, &mut statics)
            .expect("benchmark isolate should initialize");

        // resolve the entry once
        let entry = isolate
            .function_id_by_name(entry)
            .expect("benchmark entry should exist");

        Self {
            isolate,
            statics,
            heap,
            shared,
            shared_allocator,
            shared_gc,
            entry,
        }
    }

    /// Run the benchmark entry once.
    fn run(&mut self) -> Value {
        // pass the shared iteration count into every loop fixture
        let arguments = [Value::int32(ITERATIONS)];

        self.isolate
            .run_function(
                &mut self.statics,
                &mut self.heap,
                &self.shared,
                &mut self.shared_allocator,
                &self.shared_gc,
                self.entry,
                &arguments,
            )
            .expect("benchmark function should run")
    }
}

/// Benchmark VM dispatch over integer arithmetic.
fn bench_integer_arithmetic(criterion: &mut Criterion) {
    // group related scalar dispatch programs
    let mut group = dispatch_group(criterion, "vm_dispatch_arithmetic");

    // measure the typed integer loop
    bench_program(
        &mut group,
        ProgramBench {
            name: "integer_add_loop",
            entry: "integerAddLoop",
        },
    );

    group.finish();
}

/// Benchmark VM dispatch over packed vector operations.
fn bench_vector(criterion: &mut Criterion) {
    // group vector-shaped dispatch programs
    let mut group = dispatch_group(criterion, "vm_dispatch_vector");

    // measure packed i32x4 addition
    bench_program(
        &mut group,
        ProgramBench {
            name: "vector_i32x4_add_body_loop",
            entry: "vectorI32x4AddBodyLoop",
        },
    );

    // measure packed i32x4 addition with loop-carried vector state
    bench_program(
        &mut group,
        ProgramBench {
            name: "vector_i32x4_add_loop",
            entry: "vectorI32x4AddLoop",
        },
    );

    group.finish();
}

/// Benchmark VM dispatch over contiguous tensor operations.
fn bench_tensor(criterion: &mut Criterion) {
    // group tensor-shaped dispatch programs
    let mut group = dispatch_group(criterion, "vm_dispatch_tensor");

    // measure contiguous tensor addition
    bench_program(
        &mut group,
        ProgramBench {
            name: "tensor_contiguous_add_body_loop",
            entry: "tensorContiguousAddBodyLoop",
        },
    );

    // measure contiguous tensor addition with loop-carried tensor state
    bench_program(
        &mut group,
        ProgramBench {
            name: "tensor_contiguous_add_loop",
            entry: "tensorContiguousAddLoop",
        },
    );

    group.finish();
}

/// Benchmark VM dispatch over memory operations.
fn bench_memory(criterion: &mut Criterion) {
    // group memory-shaped dispatch programs
    let mut group = dispatch_group(criterion, "vm_dispatch_memory");

    // measure scalar load and store through the heap
    bench_program(
        &mut group,
        ProgramBench {
            name: "heap_load_store",
            entry: "heapLoadStore",
        },
    );

    // measure repeated managed heap allocation
    bench_program(
        &mut group,
        ProgramBench {
            name: "heap_allocate",
            entry: "heapAllocate",
        },
    );

    group.finish();
}

/// Create one benchmark group with VM dispatch throughput.
fn dispatch_group<'a>(criterion: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    // report throughput in MIR loop iterations
    let mut group = criterion.benchmark_group(name);
    group.throughput(Throughput::Elements(OPERATION_COUNT));

    group
}

/// Benchmark one MIR program entry.
fn bench_program(group: &mut BenchmarkGroup<'_, WallTime>, program: ProgramBench) {
    // read the fixture once outside the measured path
    let source = program.read();

    // rebuild the VM fixture outside the measured loop body
    group.bench_function(BenchmarkId::new(program.name, ITERATIONS), |bencher| {
        bencher.iter_batched(
            || Fixture::new(&source, program.entry),
            |mut fixture| black_box(fixture.run()),
            BatchSize::SmallInput,
        );
    });
}

/// Create one worker heap for benchmark execution.
fn heap() -> Heap {
    // use production local heap geometry
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("benchmark allocator should build"),
    );

    // keep benchmark limits uninteresting
    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("benchmark heap should build")
}

/// Create one shared heap for benchmark execution.
fn shared_heap() -> SharedHeap {
    // use production shared heap geometry
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("benchmark shared allocator should build"),
    );

    // keep benchmark limits uninteresting
    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("benchmark shared heap should build")
}

criterion_group!(
    benches,
    bench_integer_arithmetic,
    bench_vector,
    bench_tensor,
    bench_memory
);
criterion_main!(benches);
