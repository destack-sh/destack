use std::ptr::NonNull;
use std::sync::Arc;

use destack_artifact::{ConditionSet, Host, Platform, Runtime};
use destack_compiler::ProgramLinker;
use destack_core::StringPool;
use destack_heap as heap;
use destack_mir as mir;
use destack_mir::parse::{ParseOptions, Parser};
use destack_mir::{DispatchTable, DropTable, LayoutTable, TargetLayout, Tree, TypeTable};
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};
use destack_source::{DiagnosticSeverity, File, FileId, FileType, PackageId, Uri};
use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeFailure, RuntimeResult};
use crate::host::core::{HostQueue, poll_host_events};
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::time::TimerClock;
use crate::host::{
    self, HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId,
};
use crate::runtime::machine::{
    Continuation, Entry, Execution, Outcome, ProgramActivation, ProgramStorage,
};
use crate::runtime::scheduler::{Readiness, Runnable, RunnableId, ScheduledTimer, TimerDeadline};
use crate::runtime::time::Nanos;
use crate::runtime::{
    BindingCall, RuntimeHeap, Worker, WorkerId, WorkerOptions, WorkerRunOutcome, World, WorldState,
    current_runnable_scope,
};
use crate::world::{Moment, Run, RunOutcome, RuntimeId};

/// Build one resource id owned by the primary test worker.
pub(crate) fn test_resource_id(local_id: u64) -> ResourceId {
    ResourceId::new(WorkerId(1), local_id)
}

/// Build the conditions used by isolated runtime tests.
pub(crate) fn test_conditions() -> Arc<ConditionSet> {
    Arc::new(ConditionSet {
        modes: Default::default(),
        roles: Default::default(),
        features: Default::default(),
        tags: Default::default(),
        target: Some("test".to_string()),
        product: None,
        role: None,
        labels: Default::default(),
        stage: None,
        platform: Platform::Unknown,
        host: Host::Native,
        runtime: Runtime::Destack,
    })
}

/// Build one native binding call for runtime tests.
pub(crate) fn binding_call<'host>(
    worker: &mut Worker,
    host: &'host dyn host::Host,
    host_queue: &'host HostQueue,
    world: &mut WorldState,
) -> BindingCall<'host> {
    let is_process_main = host.is_process_main_context();

    BindingCall {
        runtime_id: worker.runtime_id,
        worker_id: worker.id,
        environment: worker.environment.clone(),
        conditions: worker.conditions.clone(),
        diagnostics: worker.diagnostics.clone(),
        binding_table: &worker.binding_table,
        host,
        host_queue,
        world: world as *mut WorldState,
        scope: current_runnable_scope(),
        is_process_main,
    }
}

const TEST_ENGINE_MIR: &str = r#"
function test.entry(): void {
b0:
    return
}

function test.task(v0: int32): int32 {
b0(v0: int32):
    yield v0 => b1(v0)
b1(v1: int32, v2: int32):
    v3: int32 = int.add v1, v2
    yield v3 => b2(v3)
b2(v4: int32, v5: int32):
    return v4
}

function test.complete(v0: int32): int32 {
b0(v0: int32):
    yield v0 => b1(v0)
b1(v1: int32, v2: int32):
    return v1
}

function test.breakpoint(v0: int32): int32 {
b0(v0: int32):
    yield v0 => b1(v0)
b1(v1: int32, v2: int32):
    breakpoint
    return v1
}
"#;

/// VM-backed test machine builder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TestMachine {
    /// MIR text used to build the VM machine.
    mir: &'static str,
    /// Optional destructor keyed by MIR type name.
    drop: Option<(&'static str, &'static str)>,
}

impl Default for TestMachine {
    fn default() -> Self {
        Self {
            mir: TEST_ENGINE_MIR,
            drop: None,
        }
    }
}

impl TestMachine {
    /// Build one test machine from explicit MIR text.
    pub(crate) const fn with_mir(mir: &'static str) -> Self {
        Self { mir, drop: None }
    }

    /// Attach one destructor by type and function name.
    pub(crate) const fn with_drop(
        mut self,
        type_name: &'static str,
        function_name: &'static str,
    ) -> Self {
        self.drop = Some((type_name, function_name));

        self
    }

    /// Build one durable program for this test machine.
    pub(crate) fn program(self) -> Arc<program::Program> {
        let (tree, target_layout, types, layouts, dispatch, mut drops, strings) =
            parse_mir(self.mir);

        // attach the destructor requested by this test machine
        if let Some((type_name, function_name)) = self.drop {
            let ty = tree
                .iter_nodes::<mir::Type>()
                .find_map(|(ty, _)| {
                    let name = types.display_name(ty)?;

                    (strings.get(name) == type_name).then_some(ty)
                })
                .unwrap_or_else(|| panic!("missing test MIR type {type_name}"));
            let function = tree
                .iter_nodes::<mir::Function>()
                .find_map(|(function_id, function)| {
                    (strings.get(function.name) == function_name).then_some(function_id)
                })
                .unwrap_or_else(|| panic!("missing test MIR function {function_name}"));
            drops.set_destructor(ty, function);
        }

        let program = ProgramLinker::new(
            PackageId::from_uri(&Uri::logical("test/runtime")),
            tree,
            target_layout,
            types,
            layouts,
            dispatch,
            drops,
            strings,
            vm::MachineOptions::test().heap,
            vm::MachineOptions::test().shared_heap,
        )
        .build()
        .expect("runtime test program should link");

        Arc::new(program)
    }

    /// Build one execution strategy for this test machine.
    pub(crate) fn execution(self) -> Execution {
        Execution::vm(vm::MachineOptions::test())
    }
}

/// Parse one MIR test input for program linking.
fn parse_mir(
    mir: &str,
) -> (
    Tree,
    TargetLayout,
    TypeTable,
    LayoutTable,
    DispatchTable,
    DropTable,
    StringPool,
) {
    let file_id = FileId::from_source_bytes(mir.as_bytes());
    let file = File::from_text(
        file_id,
        "test.mir".to_string(),
        Uri::from_string("test.mir"),
        None,
        FileType::Text,
        mir.to_string(),
    );
    let parsed = Parser::parse(&file, ParseOptions::default()).expect("test MIR should be text");
    let (tree, target_layout, types, layouts, dispatch, drops, _, _, _, strings, diagnostics) =
        parsed.into_parts();

    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        let diagnostic = diagnostics
            .iter()
            .next()
            .expect("parser should emit at least one diagnostic");

        panic!("failed to parse runtime test MIR: {diagnostic:?}");
    }

    (
        tree,
        target_layout,
        types,
        layouts,
        dispatch,
        drops,
        strings,
    )
}

/// Test harness for worker scheduling tests.
#[derive(Debug)]
pub(crate) struct TestRuntime {
    /// Test world that owns the worker lifetime.
    world: World,
    /// Wrapped worker under test.
    worker: Worker,
    /// Runtime-owned shared heap state used by the worker.
    heap: RuntimeHeap,
    /// Immutable program constant space used by the worker.
    constant_space: program::StaticImage,
    /// Runtime-owned shared static bytes used by the worker.
    shared_static: program::StaticSpace,
}

/// Test harness for multi-worker runtime scheduler tests.
#[derive(Debug)]
pub(crate) struct TestWorldRuntime {
    /// Test world that owns the runtime lifetime.
    world: World,
    /// Wrapped runtime identity under test.
    runtime_id: RuntimeId,
}

/// Test poller for worker event loop tests.
#[derive(Debug, Default)]
pub(crate) struct TestPoller;

impl HostPoller for TestPoller {
    /// Registering resources is not used by these tests.
    fn register(
        &mut self,
        _resource_id: ResourceId,
        _handle: HostHandle,
        _token: PollerToken,
        _interests: PollInterest,
        _flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Updating resources is not used by these tests.
    fn update(
        &mut self,
        _resource_id: ResourceId,
        _token: PollerToken,
        _interests: PollInterest,
        _flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Deregistering resources is not used by these tests.
    fn deregister(&mut self, _resource_id: ResourceId) -> RuntimeResult<()> {
        Ok(())
    }

    /// Return no dedicated wake handle for this test poller.
    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        None
    }

    /// Waking the test poller is a no-op.
    fn wake(&mut self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Return no host ingress.
    fn poll(&mut self, _timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        Ok(Vec::new())
    }
}

impl TestRuntime {
    /// Build one test worker runtime.
    pub(crate) fn build(options: &RuntimeOptions, machine: TestMachine) -> Self {
        let mut world =
            World::new(options, Environment::default()).expect("runtime test world should build");

        // build program and runtime-owned storage
        let program = machine.program();
        let execution = machine.execution();
        let shared = runtime_shared_heap(&world, options, program.clone());
        let constant_space = program.constants().clone();
        let shared_static = program
            .materialize_shared_statics(world.memory.clone())
            .expect("shared test statics should build");

        // create the worker over the runtime storage
        let mut worker = Worker::new_in_world(
            Environment::default(),
            options,
            test_conditions(),
            &mut world.state,
            &shared,
            WorkerOptions::default(),
            program,
            &execution,
        )
        .expect("runtime test worker should build");
        worker.binding_table.apply_runtime_defaults(options);

        // drain initial host bootstrap events for deterministic scheduler tests
        poll_host_events(world.host.as_ref(), &world.host_queue, Some(0))
            .expect("host bootstrap events should drain");

        Self {
            world,
            heap: shared,
            constant_space,
            shared_static,
            worker,
        }
    }

    /// Enqueue one task with explicit identifiers.
    pub(crate) fn enqueue_task(&mut self, task_id: u64, continuation_id: u64) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(task_id),
            continuation,
            resume_value: program::Value::Void,
        });
    }

    /// Register one timer waiter.
    pub(crate) fn add_timer_waiter(&mut self, handle: u64, continuation_id: u64) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker
            .add_timer_waiter(test_resource_id(handle), continuation, program::Value::Void)
            .expect("timer waiter should register");
    }

    /// Remove one timer waiter and return whether one waiter was present.
    pub(crate) fn remove_timer_waiter(&mut self, handle: u64) -> bool {
        self.worker
            .remove_timer_waiter(test_resource_id(handle))
            .is_some()
    }

    /// Schedule one timer in the event loop.
    pub(crate) fn schedule_timer(
        &mut self,
        handle: u64,
        fire_at_nanos: u64,
        interval_nanos: Option<u64>,
    ) {
        self.schedule_timer_on(TimerClock::Wall, handle, fire_at_nanos, interval_nanos);
    }

    /// Schedule one timer in the event loop on one explicit clock domain.
    pub(crate) fn schedule_timer_on(
        &mut self,
        clock: TimerClock,
        handle: u64,
        fire_at_nanos: u64,
        interval_nanos: Option<u64>,
    ) {
        self.worker
            .event_loop
            .schedule_timer(ScheduledTimer {
                resource_id: test_resource_id(handle),
                deadline: TimerDeadline {
                    clock,
                    at: Nanos::new(fire_at_nanos),
                },
                interval: interval_nanos.map(Nanos::new),
            })
            .expect("timer should schedule");
    }

    /// Register one readable-resource waiter.
    pub(crate) fn add_resource_waiter(&mut self, resource_id: u64, continuation_id: u64) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker
            .add_resource_waiter(
                test_resource_id(resource_id),
                Readiness::Readable,
                continuation,
                program::Value::Void,
            )
            .expect("resource waiter should register");
    }

    /// Register one host waiter.
    pub(crate) fn add_host_waiter(&mut self, kind: HostEventKind, continuation_id: u64) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker
            .add_host_waiter(kind, continuation, program::Value::Void)
            .expect("host waiter should register");
    }

    /// Enqueue one synthetic I/O wake for dispatch tests.
    pub(crate) fn enqueue_io_event(&mut self, resource_id: u64, token: u64, data: u64) {
        self.worker
            .event_loop
            .enqueue_poller_wakes(vec![PollerEvent {
                resource_id: test_resource_id(resource_id),
                source: PollerEventSource::Io,
                mask: PollerEventMask::READABLE,
                flags: PollerEventFlags::NONE,
                token: PollerToken(token),
                payload: PollerEventPayload::Io { data },
            }]);
    }

    /// Enqueue one synthetic lifecycle host wake for dispatch tests.
    pub(crate) fn enqueue_lifecycle_host_event(&mut self, state: LifecycleState) {
        self.worker
            .event_loop
            .enqueue_host_wakes(vec![HostEvent::Lifecycle(LifecycleEvent {
                source_kind: LifecycleSourceKind::Application,
                state,
            })]);
    }

    /// Tick once and fail loudly on runtime errors.
    pub(crate) fn tick(&mut self) -> bool {
        let outcome = self
            .worker
            .run_task(
                &mut self.world.state,
                &self.heap,
                &mut self.shared_static,
                &self.constant_space,
                self.world.host.as_ref(),
                &self.world.host_queue,
            )
            .expect("tick should execute runtime work");

        matches!(outcome, WorkerRunOutcome::Progressed { .. })
    }

    /// Tick until idle and fail loudly on runtime errors.
    pub(crate) fn tick_until_idle(&mut self) {
        let mut poller = TestPoller;
        self.worker
            .run_event_loop(
                &mut self.world.state,
                &self.heap,
                &mut self.shared_static,
                &self.constant_space,
                self.world.host.as_ref(),
                &self.world.host_queue,
                None,
                None,
                &mut poller,
            )
            .expect("tick until idle should complete");
    }

    /// Start runtime profiling on the wrapped worker.
    pub(crate) fn start_profile(&mut self, options: program::ProfileOptions) {
        self.worker.start_profile(options);
    }

    /// Return the wrapped worker profile.
    pub(crate) fn profile(&self) -> Option<&program::Profile> {
        self.worker.profile()
    }

    /// Allocate one worker-local test block.
    pub(crate) fn allocate(&mut self, shape: heap::AllocationShape) -> heap::HeapReference {
        let plan = self.worker.heap.options().allocation_plan(&shape);

        self.worker
            .heap
            .allocate_zeroed(plan, &shape.trace_map)
            .expect("test heap allocation should succeed")
    }

    /// Return whether one worker-local test allocation is live.
    pub(crate) fn is_live(&self, reference: heap::HeapReference) -> bool {
        self.worker.heap.is_heap_live(reference)
    }

    /// Request one full worker-local collection.
    pub(crate) fn request_full_gc(&mut self) {
        self.worker.heap.request_full_gc();
    }

    /// Advance one idle worker GC safepoint.
    pub(crate) fn step_gc(&mut self) -> RuntimeResult<Option<heap::GcAdvance>> {
        self.worker.run_safepoint(
            &mut self.world.state,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
        )
    }

    /// Run until one task completes or one timeout elapses.
    pub(crate) fn run_loop_until_task_complete(
        &mut self,
        task_id: u64,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<program::Value>> {
        let mut poller = TestPoller;

        let output = self.worker.run_event_loop(
            &mut self.world.state,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
            Some(RunnableId::new(task_id)),
            timeout_nanos,
            &mut poller,
        );

        match output {
            Err(error)
                if matches!(
                    error.as_ref(),
                    RuntimeError::Runtime {
                        reason: RuntimeFailure::EventLoopIdle { .. }
                    }
                ) =>
            {
                Ok(None)
            }
            result => result,
        }
    }

    /// Return whether the event loop has pending work.
    pub(crate) fn has_pending_work(&self) -> bool {
        self.worker.event_loop.has_pending_work()
    }

    /// Create one VM continuation that yields once when scheduled.
    pub(crate) fn yielding_continuation(&mut self, value: u64) -> Continuation {
        let value = i32::try_from(value).expect("test continuation id should fit int32");

        self.start_continuation("test.task", value)
    }

    /// Start one yielding VM function and return its continuation.
    fn start_continuation(&mut self, entry: &str, value: i32) -> Continuation {
        start_worker_continuation(
            &mut self.worker,
            self.world.host.as_ref(),
            &self.world.host_queue,
            &mut self.world.state,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            entry,
            value,
        )
    }
}

impl TestWorldRuntime {
    /// Build one test world runtime.
    pub(crate) fn build(options: &RuntimeOptions, machine: TestMachine) -> Self {
        let environment = Arc::new(Environment::default());
        let mut world = World::new(options, environment.clone()).expect("world should build");
        let program = machine.program();
        let execution = machine.execution();
        let runtime_id = world
            .spawn_runtime(environment, options, test_conditions(), program, execution)
            .expect("runtime should spawn");

        poll_host_events(world.host.as_ref(), &world.host_queue, Some(0))
            .expect("host bootstrap events should drain");

        Self { world, runtime_id }
    }

    /// Return the default worker id.
    pub(crate) fn default_worker_id(&self) -> WorkerId {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .default_worker_id()
    }

    /// Return the wrapped runtime id.
    pub(crate) fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    /// Borrow the wrapped world.
    pub(crate) fn world(&self) -> &World {
        &self.world
    }

    /// Borrow the wrapped world mutably.
    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Spawn one additional worker and return its id.
    pub(crate) fn spawn_worker(&mut self) -> WorkerId {
        self.world
            .spawn_worker(self.runtime_id, WorkerOptions::default())
            .expect("worker should spawn")
    }

    /// Run one closure with one mutable worker by id.
    pub(crate) fn with_worker_mut<R>(
        &mut self,
        worker_id: WorkerId,
        callback: impl FnOnce(&mut Worker) -> R,
    ) -> R {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        let worker = runtime
            .worker_mut(worker_id)
            .expect("worker should exist in runtime");

        callback(worker)
    }

    /// Run one world task and fail loudly on runtime errors.
    pub(crate) fn run_task(&mut self) -> RunOutcome {
        self.world
            .run(Run::Task)
            .expect("world task run should succeed")
    }

    /// Continue one stopped world runnable and fail loudly on runtime errors.
    pub(crate) fn run_continue(&mut self) -> RunOutcome {
        self.world
            .run(Run::Continue)
            .expect("world continue should succeed")
    }

    /// Allocate one shared test block and publish its allocation cache.
    pub(crate) fn allocate_shared(
        &mut self,
        shape: heap::AllocationShape,
    ) -> heap::SharedHeapReference {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.default_worker_id();

        runtime
            .with_worker(worker_id, |shared, _, _, worker| {
                let plan = shared.shared.options().allocation_plan(&shape);
                let reference = shared
                    .shared
                    .allocate_zeroed(
                        &worker.shared_mark_worker,
                        &mut worker.shared_cache,
                        plan,
                        &shape.trace_map,
                        shared.program().trace_view(),
                    )
                    .expect("shared test allocation should succeed");
                shared
                    .shared
                    .flush_allocation_cache(&mut worker.shared_cache);

                reference
            })
            .expect("worker should exist")
    }

    /// Request one shared collection cycle.
    pub(crate) fn request_shared_gc(&mut self) {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");

        runtime.heap.shared.request_gc();
    }

    /// Return whether one shared test allocation is live.
    pub(crate) fn is_shared_live(&self, reference: heap::SharedHeapReference) -> bool {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");

        runtime.heap.shared.is_heap_live(reference)
    }

    /// Return the current shared collector phase.
    pub(crate) fn shared_gc_phase(&self) -> heap::GcPhase {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");

        runtime.heap.shared.gc_phase()
    }

    /// Return the current world moment.
    pub(crate) fn moment(&self) -> Moment {
        self.world.moment()
    }

    /// Return current world wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Return current world monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.world.mono_nanos()
    }

    /// Create one completing continuation in one explicit worker.
    pub(crate) fn completing_continuation(
        &mut self,
        worker_id: WorkerId,
        value: u64,
    ) -> Continuation {
        self.continuation(worker_id, "test.complete", value)
    }

    /// Create one breakpoint continuation in one explicit worker.
    pub(crate) fn breakpoint_continuation(
        &mut self,
        worker_id: WorkerId,
        value: u64,
    ) -> Continuation {
        self.continuation(worker_id, "test.breakpoint", value)
    }

    /// Create one continuation from one explicit worker entrypoint.
    fn continuation(&mut self, worker_id: WorkerId, entry: &str, value: u64) -> Continuation {
        let value = i32::try_from(value).expect("test continuation id should fit int32");
        let World {
            state,
            runtimes,
            host,
            host_queue,
            ..
        } = &mut self.world;
        let runtime = runtimes
            .get_mut(&self.runtime_id)
            .expect("runtime should exist");

        runtime
            .with_worker(
                worker_id,
                |shared, shared_static, constant_space, worker| {
                    start_worker_continuation(
                        worker,
                        host.as_ref(),
                        host_queue,
                        state,
                        shared,
                        shared_static,
                        constant_space,
                        entry,
                        value,
                    )
                },
            )
            .expect("worker should exist in runtime")
    }
}

/// Build runtime-owned shared heap state for one test world.
pub(crate) fn runtime_shared_heap(
    world: &World,
    options: &RuntimeOptions,
    program: Arc<program::Program>,
) -> RuntimeHeap {
    RuntimeHeap::new(
        world.memory.clone(),
        world.shared_collector.clone(),
        options,
        program,
    )
    .expect("runtime shared heap should build")
}

/// Start one VM continuation in a worker test harness.
pub(crate) fn start_worker_continuation(
    worker: &mut Worker,
    host: &dyn host::Host,
    host_queue: &HostQueue,
    world: &mut WorldState,
    runtime_heap: &RuntimeHeap,
    shared_static: &mut program::StaticSpace,
    constant_space: &program::StaticImage,
    entry: &str,
    value: i32,
) -> Continuation {
    let mut call_context = binding_call(worker, host, host_queue, world);
    let Worker {
        heap: worker_heap,
        local_static,
        machine,
        profile,
        shared_cache,
        shared_mark_worker,
        ..
    } = worker;
    let context = ProgramActivation {
        state: NonNull::from(&mut call_context).cast(),
        storage: ProgramStorage {
            heap: worker_heap,
            shared_heap: runtime_heap.shared.as_ref(),
            shared_cache,
            shared_mark_worker,
            local_static,
            shared_static,
            constant_space,
        },
    };
    let args = [program::Value::int32(value)];
    let outcome = machine
        .run(
            context,
            &Entry::new(entry),
            &args,
            None,
            None,
            profile.as_mut(),
        )
        .expect("test continuation should start");

    match outcome {
        Outcome::Yielded { continuation, .. } => continuation,
        Outcome::Completed { .. } => panic!("test continuation entry should yield"),
        Outcome::Stopped { .. } => panic!("test continuation entry should yield"),
    }
}
