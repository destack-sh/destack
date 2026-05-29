use std::sync::Arc;

use destack_engine::{EngineId, StaticSpace};
use destack_heap::{
    Allocator, Heap, HeapLimits, HeapOptions, SharedAllocationCache, SharedGcWorker, SharedHeap,
    SharedHeapLimits, SharedHeapOptions,
};
use destack_mir::parse::{ParseOptions, Parser};
use destack_runtime::launch::Launch;
use destack_runtime::runtime::WorkerOptions;
use destack_runtime::runtime::engine::{Engine, Entry};
use destack_runtime::world::{RuntimeId, World};
use destack_source::FileId;
use destack_vm::{Continuation, ContinuationImage, Isolate, IsolateOptions, Outcome};
use destack_workspace::{Environment, ExecutionMode, RuntimeOptions};

/// MIR program used by footprint scenarios.
const VM_PROGRAM: &str = r#"
function bench.entry(): void {
b0:
    return
}

function bench.yieldFrame(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    yield v1, b1(v0)
b1(v2: ref<int32, raw, readonly, space(frame)>, v3: int32):
    v4: int32 = load v2
    return v4
}
"#;

/// Runtime-level footprint scenario.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeScenario {
    /// Runtime options shared by every run.
    options: RuntimeOptions,
    /// Ambient launch environment.
    environment: Arc<Environment>,
}

impl RuntimeScenario {
    /// Create one default runtime footprint scenario.
    pub(crate) fn new() -> Self {
        let mut options = RuntimeOptions::default();
        options.execution.mode = ExecutionMode::Strict;

        Self {
            options,
            environment: Arc::new(Environment::default()),
        }
    }

    /// Create one empty world.
    pub(crate) fn world(&self) -> World {
        World::new(&self.options, self.environment.clone()).expect("footprint world should build")
    }

    /// Spawn one VM runtime into an existing world.
    pub(crate) fn spawn_runtime(&self, world: &mut World, engine: Engine) -> RuntimeId {
        world
            .spawn_runtime(self.environment.clone(), &self.options, engine)
            .expect("footprint runtime should spawn")
    }

    /// Create one world with one VM runtime.
    pub(crate) fn world_with_runtime(&self) -> (World, RuntimeId) {
        let mut world = self.world();
        let engine = self.engine();
        let runtime_id = self.spawn_runtime(&mut world, engine);

        (world, runtime_id)
    }

    /// Spawn one VM worker into an existing runtime.
    pub(crate) fn spawn_worker(&self, world: &mut World, runtime_id: RuntimeId, engine: Engine) {
        world
            .spawn_worker(runtime_id, WorkerOptions::default(), engine)
            .expect("footprint worker should spawn");
    }

    /// Launch one empty VM runtime and drain it.
    pub(crate) fn launch(&self) {
        Launch::new(
            self.options.clone(),
            self.environment.clone(),
            self.engine(),
            Entry::new("bench.entry"),
        )
        .run()
        .expect("footprint launch should run");
    }

    /// Build one VM engine.
    pub(crate) fn engine(&self) -> Engine {
        Engine::from(build_isolate())
    }
}

/// VM-level footprint scenario.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VmScenario;

impl VmScenario {
    /// Create one VM footprint scenario.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Build one VM isolate.
    pub(crate) fn isolate(self) -> Isolate {
        build_isolate()
    }

    /// Build one worker-local heap.
    pub(crate) fn local_heap(self) -> Heap {
        heap()
    }

    /// Build one shared heap.
    pub(crate) fn shared_heap(self) -> SharedHeap {
        shared_heap()
    }

    /// Build one initialized VM machine.
    pub(crate) fn machine(self) -> VmMachine {
        VmMachine::new()
    }
}

/// Initialized VM machine.
pub(crate) struct VmMachine {
    /// The isolate under measurement.
    isolate: Isolate,
    /// Worker static byte space.
    statics: StaticSpace,
    /// Worker-local heap.
    heap: Heap,
    /// Runtime shared heap.
    shared: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: SharedAllocationCache,
    /// Shared collector worker.
    shared_gc: SharedGcWorker,
}

impl VmMachine {
    /// Create one initialized VM machine.
    pub(crate) fn new() -> Self {
        let mut isolate = build_isolate();
        let mut statics = StaticSpace::empty();
        let heap = heap();
        let shared = shared_heap();
        let shared_gc = shared.register_collector_worker();
        let shared_cache = shared.allocation_cache();

        isolate
            .initialize(&heap, &shared, &mut statics)
            .expect("footprint isolate should initialize");

        Self {
            isolate,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
        }
    }

    /// Run one yielding VM entry and return its continuation.
    pub(crate) fn yield_once(&mut self) -> Continuation {
        let entry = self
            .isolate
            .function_id_by_name("bench.yieldFrame")
            .expect("footprint entry should exist");
        let outcome = self
            .isolate
            .run_function_yielding(
                &mut self.statics,
                &mut self.heap,
                &self.shared,
                &mut self.shared_cache,
                &self.shared_gc,
                entry,
                &[],
            )
            .expect("footprint continuation should yield");

        match outcome {
            Outcome::Yielded { continuation, .. } => continuation,
            Outcome::Completed { value } => panic!("expected yield, got {value:?}"),
        }
    }

    /// Capture one continuation image.
    pub(crate) fn continuation_image(&self, continuation: &Continuation) -> ContinuationImage {
        self.isolate
            .continuation_image(continuation)
            .expect("footprint continuation image should capture")
    }
}

/// Build one VM isolate from the footprint MIR.
fn build_isolate() -> Isolate {
    let (tree, strings) = Parser::parse(FileId::new(0), VM_PROGRAM, ParseOptions::default())
        .finish()
        .expect("footprint MIR should parse");

    Isolate::build_with_options(EngineId::new(1), tree, strings, IsolateOptions::unbounded())
        .expect("footprint isolate should build")
}

/// Create one worker heap.
fn heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("footprint allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("footprint heap should build")
}

/// Create one shared heap.
fn shared_heap() -> SharedHeap {
    let options = SharedHeapOptions::default();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("footprint shared page allocator should build"),
    );

    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("footprint shared heap should build")
}
