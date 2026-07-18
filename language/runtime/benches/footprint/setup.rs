use std::sync::Arc;

use destack_artifact::{ConditionSet, Host, Platform, Runtime};
use destack_compiler::ProgramLinker;
use destack_heap::{
    AllocationCache, DEFAULT_MEMORY_MAP_SIZE_BYTES, Heap, HeapLimits, HeapOptions, SharedHeap,
    SharedHeapLimits, SharedHeapOptions, SharedMarkWorker,
};
use destack_memory::MemoryMap;
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
    /// Runtime conditions shared by every run.
    conditions: Arc<ConditionSet>,
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
            conditions: Arc::new(ConditionSet {
                modes: Default::default(),
                roles: Default::default(),
                features: Default::default(),
                tags: Default::default(),
                target: Some("footprint".to_string()),
                product: None,
                role: None,
                labels: Default::default(),
                stage: None,
                platform: Platform::Unknown,
                host: Host::Native,
                runtime: Runtime::Destack,
            }),
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
            .spawn_runtime(
                self.environment.clone(),
                &self.options,
                self.conditions.clone(),
                program,
                execution,
            )
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
            self.conditions.clone(),
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
        let options = MachineOptions::unbounded();
        let program = build_program(&options);
        let memory = memory(options.heap.page_size_bytes);
        let statics = program
            .materialize_local_statics(memory.clone())
            .expect("footprint local statics should build");
        let shared_statics = program
            .materialize_shared_statics(memory.clone())
            .expect("footprint shared statics should build");
        let heap = heap(memory.clone());
        let shared = shared_heap(memory.clone());
        let shared_mark_worker = shared.register_mark_worker();
        let shared_cache = shared.allocation_cache();
        let machine =
            Machine::new(program, memory, options).expect("footprint machine should build");

        machine
            .require_heap_compatibility(&heap, &shared)
            .expect("footprint heaps should match the machine");

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
                None,
                None,
                None,
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
    let program = build_program(&options);
    let memory = memory(options.heap.page_size_bytes);

    Machine::new(program, memory, options).expect("footprint machine should build")
}

/// Build one executable footprint program.
fn build_program(options: &MachineOptions) -> Arc<Program> {
    let parsed = Parser::parse(FileId::new(0), VM_PROGRAM, ParseOptions::default());
    let (tree, target_layout, types, layouts, dispatch, drops, _, _, _, strings, diagnostics) =
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
        drops,
        strings,
        options.heap.clone(),
        options.shared_heap.clone(),
    )
    .build()
    .expect("footprint program should link");

    Arc::new(program)
}

/// Create one worker heap.
fn heap(memory: Arc<MemoryMap>) -> Heap {
    let options = HeapOptions::local();

    Heap::new(memory, HeapLimits::default(), options).expect("footprint heap should build")
}

/// Create one shared heap.
fn shared_heap(memory: Arc<MemoryMap>) -> SharedHeap {
    let options = SharedHeapOptions::default();

    SharedHeap::new(memory, SharedHeapLimits::default(), options)
        .expect("footprint shared heap should build")
}

/// Reserve one world memory map for footprint measurement.
fn memory(page_size_bytes: usize) -> Arc<MemoryMap> {
    Arc::new(
        MemoryMap::reserve(DEFAULT_MEMORY_MAP_SIZE_BYTES, page_size_bytes)
            .expect("footprint memory map should reserve"),
    )
}
