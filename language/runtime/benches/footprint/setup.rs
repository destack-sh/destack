use std::sync::Arc;

use destack_artifact::{
    ConditionSet, EmitFormat, Host, MirLowered, MirOptimized, Platform, Runtime,
};
use destack_compiler::{BytecodeEmitter, LayoutBuilder, ObjectEmitter, ProgramLinker};
use destack_core::StringPool;
use destack_heap::{
    AllocationCache, AllocationPlan, DEFAULT_HEAP_PAGE_SIZE_BYTES, DEFAULT_MEMORY_MAP_SIZE_BYTES,
    Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits, SharedHeapOptions,
    SharedMarkWorker,
};
use destack_memory::MemoryMap;
use destack_mir as mir;
use destack_program as program;
use destack_program::{Activation, FunctionId, Memory, Outcome, StaticSpace, Value};
use destack_repository::{Environment, ExecutionMode, RuntimeOptions};
use destack_runtime::binding::BindingTable;
use destack_runtime::launch::Launch;
use destack_runtime::machine::{Engine, Entry};
use destack_runtime::worker::WorkerOptions;
use destack_runtime::world::{RuntimeId, World};
use destack_source::{
    DiagnosticSeverity, File, FileId, FileType, ModuleId, PackageId, TargetId, Uri,
};
use destack_vm::{Error, Machine, MachineLimits, Result};

const ENTRY: &str = "bench.entry";
const PROGRAM: &str = r#"
export function bench.entry(): void {
entry:
    return
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

/// VM-level footprint setup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VmSetup;

/// Initialized VM machine.
pub(crate) struct VmMachine {
    /// Immutable Program retained by this machine.
    program: Arc<program::Program>,
    /// Footprint entry function.
    entry: FunctionId,
    /// Worker-local continuation storage.
    continuations: program::ContinuationTable,
    /// Machine under measurement.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[Option<AllocationPlan>]>,
    /// Runtime services used by direct VM execution.
    runtime: VmRuntime,
    /// Worker static byte space.
    local_static: StaticSpace,
    /// Runtime shared static byte space.
    shared_static: StaticSpace,
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
    pub(crate) fn spawn_runtime(&self, world: &mut World, engine: Engine) -> RuntimeId {
        world
            .spawn_runtime(
                self.environment.clone(),
                &self.options,
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

    /// Launch one empty VM runtime and drain it.
    pub(crate) fn launch(&self) {
        Launch::new(
            self.options.clone(),
            self.conditions.clone(),
            self.environment.clone(),
            Arc::new(BindingTable::new()),
            self.engine(),
            Entry::new(ENTRY),
        )
        .run()
        .expect("footprint launch should run");
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
        let memory = self.memory();

        Machine::new(program, memory, MachineLimits::unbounded())
            .expect("footprint machine should build")
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
        let shared = setup.shared_heap(memory.clone());
        let shared_mark_worker = shared.register_mark_worker();
        let shared_cache = shared.allocation_cache();
        let local_static = program
            .materialize_local_statics(memory.clone())
            .expect("footprint local statics should build");
        let shared_static = program
            .materialize_shared_statics(memory.clone())
            .expect("footprint shared statics should build");
        let allocation_plans = program
            .plan_allocations(heap.options(), shared.options())
            .expect("footprint allocation plans should build")
            .into();
        let entry = program
            .function_id_by_name(ENTRY)
            .expect("footprint entry should exist");
        let machine = Machine::new(program.clone(), memory, MachineLimits::unbounded())
            .expect("footprint machine should build");

        Self {
            program,
            entry,
            continuations: program::ContinuationTable::default(),
            machine,
            allocation_plans,
            runtime: VmRuntime,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
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
                heap: &mut self.heap,
                shared_heap: &self.shared,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static: &mut self.shared_static,
                constant_space: self.program.constants(),
            },
        };
        let outcome = self
            .machine
            .run(
                &mut self.continuations,
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
            Outcome::Awaited { .. } | Outcome::Yielded { .. } => {
                panic!("footprint entry suspended")
            }
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
    fn poll(
        &mut self,
        _memory: program::Memory<'_>,
        _roots: &mut dyn program::RootSource<Error = Self::Error>,
    ) -> Result<program::Poll> {
        Ok(program::Poll::Continue)
    }

    /// Reject runtime bindings outside runtime footprint execution.
    fn call_binding(
        &mut self,
        _memory: Memory<'_>,
        _context: program::Context,
        _binding: &program::Binding,
        _arguments: &[program::Word],
        _result: &mut [program::Word],
    ) -> Result<()> {
        unreachable!("runtime footprint execution does not call runtime bindings")
    }

    /// Reject waiter queues outside the runtime scheduler.
    fn queue_waiter(&mut self, waiter: program::Waiter, _value: Value) -> Result<bool> {
        Err(program::Error::UndefinedWaiter { waiter }.into())
    }

    /// Reject waiter cancellation outside the runtime scheduler.
    fn cancel_waiter(&mut self, waiter: program::Waiter) -> Result<bool> {
        Err(program::Error::UndefinedWaiter { waiter }.into())
    }

    /// Reject resolved tasks outside the runtime scheduler.
    fn resolve_task(&mut self, _value: Value) -> program::Task {
        unreachable!("footprint execution does not create tasks")
    }

    /// Reject eager tasks outside the runtime scheduler.
    fn start_task(&mut self) -> program::Task {
        unreachable!("footprint execution does not create tasks")
    }

    /// Reject task cancellation requests outside the runtime scheduler.
    fn cancel_task(&mut self, task: program::Task) -> Result<()> {
        Err(program::Error::UndefinedTask { task }.into())
    }

    /// Reject task suspension outside the runtime scheduler.
    fn suspend_task(
        &mut self,
        task: program::Task,
        continuation: program::Continuation,
    ) -> std::result::Result<program::Waiter, (Self::Error, program::Continuation)> {
        Err((program::Error::UndefinedTask { task }.into(), continuation))
    }

    /// Reject task waiting outside the runtime scheduler.
    fn park_task(&mut self, task: program::Task, _waiter: program::Waiter) -> Result<()> {
        Err(program::Error::UndefinedTask { task }.into())
    }

    /// Reject task cancellation queries outside the runtime scheduler.
    fn is_task_cancelled(&mut self, task: program::Task) -> Result<bool> {
        Err(program::Error::UndefinedTask { task }.into())
    }

    /// Reject task detachment outside the runtime scheduler.
    fn detach_task(&mut self, task: program::Task) -> Result<()> {
        Err(program::Error::UndefinedTask { task }.into())
    }

    /// Reject terminal task outcomes outside the runtime scheduler.
    fn finish_task(&mut self, task: program::Task, _outcome: program::TaskOutcome) -> Result<()> {
        Err(program::Error::UndefinedTask { task }.into())
    }
}

impl VmSetup {
    /// Build one executable footprint program.
    fn program(self) -> Arc<program::Program> {
        let package = PackageId::new(0);
        let module = ModuleId::new(package, 0);
        let target = TargetId::new(package, "footprint");
        let (optimized, strings) = self.optimize(module);

        // emit one relocatable object through the production compiler path
        let emitter = ObjectEmitter::new(module, &optimized, [])
            .expect("footprint MIR should emit object metadata");
        let bytecode = BytecodeEmitter::new(module, &optimized, &emitter)
            .emit()
            .expect("footprint MIR should emit bytecode");
        let object = Arc::new(emitter.build(bytecode));

        // link the object into one executable Program
        let program = ProgramLinker::new(
            package,
            target,
            EmitFormat::Bytecode,
            vec![(module, object)],
            &strings,
        )
        .expect("footprint object should initialize its linker")
        .link()
        .expect("footprint object should link");

        Arc::new(program)
    }

    /// Parse and complete the footprint MIR.
    fn optimize(self, module: ModuleId) -> (MirOptimized, StringPool) {
        let file = File::from_text(
            FileId::from_source_bytes(PROGRAM.as_bytes()),
            "<footprint.dsm>".to_string(),
            Uri::from_string("<footprint.dsm>"),
            None,
            FileType::Text,
            PROGRAM.to_string(),
        );
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("footprint MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            panic!("failed to parse footprint MIR: {:?}", parsed.diagnostics);
        }
        let (tree, target, types, layouts, dispatch, drops, memory, effects, profile, strings, _) =
            parsed.into_parts();
        let lowered = MirLowered {
            tree,
            target,
            types,
            layouts,
            dispatch,
            drops,
            memory,
            effects,
            profile,
            initializer: None,
        };

        // complete physical layouts required by object emission
        let mut tree = lowered.tree;
        let mut layouts = lowered.layouts;
        let mut builder = LayoutBuilder::new(module, &mut tree, &mut layouts, lowered.target);
        builder
            .layout_reachable_types()
            .expect("footprint MIR layouts should build");
        let optimized = MirOptimized {
            tree,
            target: lowered.target,
            types: lowered.types,
            layouts,
            dispatch: lowered.dispatch,
            drops: lowered.drops,
            memory: lowered.memory,
            effects: lowered.effects,
            profile: lowered.profile,
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
