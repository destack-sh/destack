use std::sync::Arc;

use destack_artifact::{ConditionSet, Host, MirLowered, MirOptimized, Platform, Runtime};
use destack_compiler::{BytecodeEmitter, ObjectEmitter, ProgramLinker};
use destack_core::StringPool;
use destack_heap::{
    AllocationCache, AllocationPlan, DEFAULT_HEAP_PAGE_SIZE_BYTES, DEFAULT_MEMORY_MAP_SIZE_BYTES,
    Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits, SharedHeapOptions,
    SharedMarkWorker,
};
use destack_memory::{MemoryMap, MemoryRange};
use destack_mir as mir;
use destack_program as program;
use destack_program::{Activation, FunctionId, Memory, Outcome, StaticSpace, Value};
use destack_repository::{Environment, RuntimeOptions, WorldOptions};
use destack_runtime::binding::BindingTable;
use destack_runtime::machine::{Engine, Entry};
use destack_runtime::runtime::RuntimeId;
use destack_runtime::worker::WorkerOptions;
use destack_runtime::world::{RunOutcome, World};
use destack_source::{DiagnosticSeverity, File, FileId, FileType, ModuleId, PackageId, Uri};
use destack_vm::{Error, Machine, MachineLimits, Result};

/// Exported entrypoint measured by the footprint benchmarks.
const ENTRY: &str = "bench.entry";
/// Minimal Program executed by the footprint benchmarks.
const PROGRAM: &str = r#"
export function bench.entry(): void {
entry:
    return
}
"#;

/// Runtime-level footprint setup.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeSetup {
    /// World options shared by every run.
    world_options: WorldOptions,
    /// Runtime options shared by every run.
    runtime_options: RuntimeOptions,
    /// Runtime conditions shared by every run.
    conditions: Arc<ConditionSet>,
    /// Ambient runtime environment.
    environment: Arc<Environment>,
}

/// VM-level footprint setup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VmSetup;

/// Initialized VM machine.
pub(crate) struct VmMachine {
    /// Footprint entry function.
    entry: FunctionId,
    /// Reusable footprint execution fiber.
    fiber: destack_vm::Fiber,
    /// Machine under measurement.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[AllocationPlan]>,
    /// Runtime services used by direct VM execution.
    runtime: VmRuntime,
    /// Runtime constant byte space.
    constant_space: StaticSpace,
    /// Worker static byte space.
    local_static: StaticSpace,
    /// Runtime shared static byte space.
    shared_static: StaticSpace,
    /// The request word the machine polls at safepoints.
    handshake: program::Handshake,
    /// Worker-local heap.
    heap: Heap,
    /// Runtime shared heap.
    shared: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// Shared mark worker.
    shared_mark_worker: SharedMarkWorker,
}

impl RuntimeSetup {
    /// Create one default runtime footprint setup.
    pub(crate) fn new() -> Self {
        Self {
            world_options: WorldOptions::default(),
            runtime_options: RuntimeOptions::default(),
            conditions: Arc::new(ConditionSet {
                modes: Default::default(),
                roles: Default::default(),
                features: Default::default(),
                tags: Default::default(),
                target: Some("footprint".to_string()),
                product: None,
                role: None,
                labels: Default::default(),
                platform: Platform::Unknown,
                host: Host::Native,
                runtime: Runtime::Destack,
            }),
            environment: Arc::new(Environment::default()),
        }
    }

    /// Create one empty world.
    pub(crate) fn world(&self) -> World {
        World::new(&self.world_options, self.environment.clone())
            .expect("footprint world should build")
    }

    /// Spawn one VM runtime into an existing world.
    pub(crate) fn spawn_runtime(&self, world: &mut World, engine: Engine) -> RuntimeId {
        world
            .spawn_runtime(
                self.environment.clone(),
                &self.runtime_options,
                self.conditions.clone(),
                Arc::new(BindingTable::new()),
                engine,
            )
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
    pub(crate) fn spawn_worker(&self, world: &mut World, runtime_id: RuntimeId) {
        world
            .spawn_worker(runtime_id, WorkerOptions::default())
            .expect("footprint worker should spawn");
    }

    /// Execute one empty VM runtime and drain its World.
    pub(crate) fn execute(&self) {
        let (mut world, runtime_id) = self.world_with_runtime();
        world
            .invoke(runtime_id, &Entry::new(ENTRY), &[])
            .expect("footprint entrypoint should run");

        let outcome = world.drain().expect("footprint World should drain");
        assert_eq!(outcome, RunOutcome::Idle);
    }

    /// Build one durable runtime program.
    pub(crate) fn program(&self) -> Arc<program::Program> {
        VmSetup::new().program()
    }

    /// Build worker machine construction state.
    pub(crate) fn engine(&self) -> Engine {
        Engine::new(self.program(), MachineLimits::unbounded())
    }
}

impl VmSetup {
    /// Create one VM footprint setup.
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Build one VM machine before runtime storage initialization.
    pub(crate) fn build_machine(self) -> Machine {
        let program = self.program();

        Machine::new(program, MachineLimits::unbounded()).expect("footprint machine should build")
    }

    /// Build one initialized VM machine with runtime storage.
    pub(crate) fn machine(self) -> VmMachine {
        VmMachine::new(self)
    }
}

impl VmMachine {
    /// Create one initialized VM machine.
    pub(crate) fn new(setup: VmSetup) -> Self {
        let program = setup.program();
        let memory = setup.memory();
        let heap = setup.heap(memory.clone());
        let mut shared = setup.shared_heap(memory.clone());
        let shared_mark_worker = shared.register_mark_worker();
        let shared_cache = shared.allocation_cache();
        let (constant_space, shared_static) = program
            .materialize_runtime_statics(memory.clone())
            .expect("footprint runtime statics should build");
        let local_static = program
            .materialize_local_statics(memory.clone(), &constant_space, &shared_static)
            .expect("footprint local statics should build");
        shared.set_constant_range(MemoryRange {
            offset: constant_space.offset(),
            byte_len: constant_space.byte_len(),
        });
        let allocation_plans = program
            .plan_allocations(heap.options(), shared.options())
            .expect("footprint allocation plans should build")
            .into();
        let entry = program
            .function_id_by_name(ENTRY)
            .expect("footprint entry should exist");
        let machine = Machine::new(program.clone(), MachineLimits::unbounded())
            .expect("footprint machine should build");
        let fiber = machine
            .reserve_fiber(memory)
            .expect("footprint fiber should reserve");

        Self {
            entry,
            fiber,
            machine,
            allocation_plans,
            runtime: VmRuntime,
            constant_space,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            handshake: program::Handshake::new(),
        }
    }

    /// Run the empty VM entry once.
    pub(crate) fn run(&mut self) -> Value {
        let mut context = program::Context::empty();
        let activation = Activation {
            runtime: &mut self.runtime,
            context: &mut context,
            memory: Memory {
                allocation_plans: &self.allocation_plans,
                local_heap: &mut self.heap,
                shared_heap: &self.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: &mut self.shared_static,
                constants: &self.constant_space,
                handshake: &self.handshake,
            },
        };
        let outcome = self
            .machine
            .run(
                &mut self.fiber,
                activation,
                self.entry,
                None,
                &[],
                None,
                None,
                None,
            )
            .expect("footprint entry should execute");

        match outcome {
            Outcome::Completed { value } => value,
            Outcome::Cancelled => panic!("footprint entry cancelled"),
            Outcome::Stopped { reason } => panic!("footprint entry stopped: {reason:?}"),
            Outcome::Parked => panic!("footprint entry parked"),
        }
    }
}

/// Runtime services for direct footprint VM execution.
struct VmRuntime;

impl program::Runtime for VmRuntime {
    type Error = Error;

    /// Return whether execution must yield at the current runtime poll.
    fn is_poll_requested(&self) -> bool {
        false
    }

    /// Continue footprint execution after one impossible poll request.
    fn poll(&mut self, _memory: program::Memory<'_>) -> Result<program::Poll> {
        Ok(program::Poll::Continue)
    }

    /// Reject runtime bindings outside runtime footprint execution.
    fn call_binding(
        &mut self,
        _memory: Memory<'_>,
        _context: program::Context,
        _fiber_id: Option<program::FiberId>,
        _binding: &program::Binding,
        _arguments: &[program::Word],
        _result: &mut [program::Word],
    ) -> Result<()> {
        unreachable!("runtime footprint execution does not call runtime bindings")
    }

    /// Reject fiber parks outside the runtime scheduler.
    fn park(&mut self, fiber_id: program::FiberId) -> Result<program::Park> {
        Err(program::Error::UndefinedFiber { fiber_id }.into())
    }
}

impl VmSetup {
    /// Build one executable footprint program.
    fn program(self) -> Arc<program::Program> {
        let package = PackageId::new(0);
        let module = ModuleId::new(package, 0);
        let (optimized, strings) = self.build_mir();

        // emit one relocatable object through the production compiler path
        let mut analyses = mir::ModuleCache::with_target_layout(optimized.target);
        let emitter = ObjectEmitter::new(module, &optimized, [], &mut analyses)
            .expect("footprint MIR should emit object metadata");
        let bytecode = BytecodeEmitter::new(module, &optimized, &emitter)
            .emit(&mut analyses)
            .expect("footprint MIR should emit bytecode");
        let object = Arc::new(emitter.bytecode(bytecode).build());

        // link the object into one executable Program
        let program = ProgramLinker::new(package, vec![(module, object)], &strings)
            .expect("footprint object should initialize its linker")
            .link()
            .expect("footprint object should link");

        Arc::new(program)
    }

    /// Build the footprint MIR snapshots.
    fn build_mir(self) -> (MirOptimized, StringPool) {
        let file = File::from_text(
            FileId::from_source_bytes(PROGRAM.as_bytes()),
            "<footprint.dsm>".to_string(),
            Uri::from_string("<footprint.dsm>"),
            None,
            FileType::Text,
            PROGRAM.to_string(),
        )
        .expect("footprint MIR should load");
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("footprint MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            panic!("failed to parse footprint MIR: {:?}", parsed.diagnostics);
        }
        let (tree, target, layouts, dispatch, drops, effects, profile, strings, _) =
            parsed.into_parts();
        let tree = std::sync::Arc::new(tree);
        let lowered = MirLowered {
            tree: std::sync::Arc::clone(&tree),
            target,
            layouts,
            dispatch,
            drops,
            witnesses: mir::WitnessTable::default(),
            effects,
            profile,
            initializer: None,
        };

        // complete physical layouts required by object emission
        let tree = lowered.tree.clone();
        let mut layouts = lowered.layouts.clone();
        let mut builder = mir::LayoutBuilder::new(&tree, &mut layouts, lowered.target);
        builder
            .layout_reachable_types()
            .expect("footprint MIR layouts should build");
        let optimized = MirOptimized {
            target: lowered.target,
            initializer: lowered.initializer,
            tree: (*tree).clone(),
            layouts,
            dispatch: lowered.dispatch.clone(),
            drops: lowered.drops.clone(),
            effects: lowered.effects.clone(),
            profile: lowered.profile.clone(),
        };

        (optimized, strings)
    }

    /// Create one worker heap.
    fn heap(self, memory: Arc<MemoryMap>) -> Heap {
        Heap::new(memory, HeapLimits::default(), HeapOptions::local())
            .expect("footprint heap should build")
    }

    /// Create one shared heap.
    fn shared_heap(self, memory: Arc<MemoryMap>) -> SharedHeap {
        SharedHeap::new(
            memory,
            SharedHeapLimits::default(),
            SharedHeapOptions::default(),
        )
        .expect("footprint shared heap should build")
    }

    /// Reserve one world memory map for footprint measurement.
    fn memory(self) -> Arc<MemoryMap> {
        Arc::new(
            MemoryMap::reserve(DEFAULT_MEMORY_MAP_SIZE_BYTES, DEFAULT_HEAP_PAGE_SIZE_BYTES)
                .expect("footprint memory map should reserve"),
        )
    }
}
