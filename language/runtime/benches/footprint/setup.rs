use std::sync::Arc;

use destack_compiler::ProgramLinker;
use destack_heap::{
    AllocationCache, Allocator, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SharedMarkWorker,
};
use destack_mir::parse::{ParseOptions, Parser};
use destack_program::{Program, StaticSpace};
use destack_repository::{Environment, ExecutionMode, RuntimeOptions};
use destack_runtime::launch::Launch;
use destack_runtime::runtime::WorkerOptions;
use destack_runtime::runtime::machine::{Entry, Execution};
use destack_runtime::world::{RuntimeId, World};
use destack_source::{DiagnosticSeverity, FileId, PackageId, Uri};
use destack_vm::{Continuation, Machine, MachineOptions, Outcome};

/// MIR program used by footprint setups.
const VM_PROGRAM: &str = r#"
function bench.entry(): void {
b0:
    return
}

function bench.yieldFrame(): int32 {
b0:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    yield v1 => b1(v0)
b1(v2: ref<int32, raw, mutable, space(frame)>, v3: int32):
    v4: int32 = load v2
    return v4
}
"#;

/// Runtime-level footprint setup.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeSetup {
    /// Runtime options shared by every run.
    options: RuntimeOptions,
    /// Ambient launch environment.
    environment: Arc<Environment>,
}

impl RuntimeSetup {
    /// Create one default runtime footprint setup.
    pub(crate) fn new() -> Self {
        let options = RuntimeOptions {
            mode: ExecutionMode::Strict,
            ..Default::default()
        };

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
    pub(crate) fn spawn_runtime(
        &self,
        world: &mut World,
        program: Arc<Program>,
        execution: Execution,
    ) -> RuntimeId {
        world
            .spawn_runtime(self.environment.clone(), &self.options, program, execution)
            .expect("footprint runtime should spawn")
    }

    /// Create one world with one VM runtime.
    pub(crate) fn world_with_runtime(&self) -> (World, RuntimeId) {
        let mut world = self.world();
        let program = self.program();
        let execution = self.execution();
        let runtime_id = self.spawn_runtime(&mut world, program, execution);

        (world, runtime_id)
    }

    /// Spawn one VM worker into an existing runtime.
    pub(crate) fn spawn_worker(&self, world: &mut World, runtime_id: RuntimeId) {
        world
            .spawn_worker(runtime_id, WorkerOptions::default())
            .expect("footprint worker should spawn");
    }

    /// Launch one empty VM runtime and drain it.
    pub(crate) fn launch(&self) {
        Launch::new(
            self.options.clone(),
            self.environment.clone(),
            self.program(),
            self.execution(),
            Entry::new("bench.entry"),
        )
        .run()
        .expect("footprint launch should run");
    }

    /// Build one durable runtime program.
    pub(crate) fn program(&self) -> Arc<Program> {
        build_machine().program_handle()
    }

    /// Build one runtime execution strategy.
    pub(crate) fn execution(&self) -> Execution {
        Execution::vm(MachineOptions::unbounded())
    }
}

/// VM-level footprint setup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VmSetup;

impl VmSetup {
    /// Create one VM footprint setup.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Build one VM machine before runtime memory initialization.
    pub(crate) fn build_machine(self) -> Machine {
        build_machine()
    }

    /// Build one worker-local heap.
    pub(crate) fn local_heap(self) -> Heap {
        heap()
    }

    /// Build one shared heap.
    pub(crate) fn shared_heap(self) -> SharedHeap {
        shared_heap()
    }

    /// Build one initialized VM machine with runtime memory.
    pub(crate) fn machine(self) -> VmMachine {
        VmMachine::new()
    }
}

/// Initialized VM machine.
pub(crate) struct VmMachine {
    /// The machine under measurement.
    machine: Machine,
    /// Worker static byte space.
    statics: StaticSpace,
    /// Runtime shared static byte space.
    shared_statics: StaticSpace,
    /// Worker-local heap.
    heap: Heap,
    /// Runtime shared heap.
    shared: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// Shared mark worker.
    shared_mark_worker: SharedMarkWorker,
}

impl VmMachine {
    /// Create one initialized VM machine.
    pub(crate) fn new() -> Self {
        let mut machine = build_machine();
        let mut statics = StaticSpace::empty();
        let mut shared_statics = StaticSpace::empty();
        let heap = heap();
        let shared = shared_heap();
        let shared_mark_worker = shared.register_mark_worker();
        let shared_cache = shared.allocation_cache();

        machine
            .initialize(&heap, &shared, &mut statics, &mut shared_statics)
            .expect("footprint machine should initialize");

        Self {
            machine,
            statics,
            shared_statics,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
        }
    }

    /// Run one yielding VM entry and return its continuation.
    pub(crate) fn yield_once(&mut self) -> Continuation {
        let entry = self
            .machine
            .function_id_by_name("bench.yieldFrame")
            .expect("footprint entry should exist");
        let outcome = self
            .machine
            .run_function_yielding(
                &mut self.statics,
                &mut self.shared_statics,
                &mut self.heap,
                &self.shared,
                &mut self.shared_cache,
                &self.shared_mark_worker,
                entry,
                &[],
            )
            .expect("footprint continuation should yield");

        match outcome {
            Outcome::Yielded { continuation, .. } => continuation,
            Outcome::Completed { value } => panic!("expected yield, got {value:?}"),
            Outcome::Stopped {
                reason,
                continuation: _,
            } => panic!("expected yield, got stop {reason:?}"),
        }
    }
}

/// Build one VM machine from the footprint MIR.
fn build_machine() -> Machine {
    let options = MachineOptions::unbounded();
    let parsed = Parser::parse(FileId::new(0), VM_PROGRAM, ParseOptions::default());
    let (tree, target_layout, types, layouts, dispatch, _, _, _, _, strings, diagnostics) =
        parsed.into_parts();

    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        let Some(diagnostic) = diagnostics.iter().next() else {
            panic!("parser reported errors without diagnostics");
        };

        panic!("failed to parse footprint MIR: {diagnostic:?}");
    }

    let program = ProgramLinker::new(
        PackageId::from_uri(&Uri::logical("bench/footprint")),
        tree,
        target_layout,
        types,
        layouts,
        dispatch,
        strings,
        options.heap.clone(),
        options.shared_heap.clone(),
    )
    .build()
    .expect("footprint program should link");

    Machine::new(Arc::new(program), options).expect("footprint machine should build")
}

/// Create one worker heap.
fn heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("footprint allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("footprint heap should build")
}

/// Create one shared heap.
fn shared_heap() -> SharedHeap {
    let options = SharedHeapOptions::default();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("footprint shared page allocator should build"),
    );

    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("footprint shared heap should build")
}
