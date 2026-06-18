use std::sync::Arc;

use destack_mir::TraceTable;
use destack_mir::parse::{ParseOptions, Parser};
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};
use destack_source::FileId;
use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeFailure, RuntimeResult};
use crate::host::core::{Host, HostQueue, poll_host_events};
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::time::TimerClock;
use crate::host::{
    HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId,
};
use crate::runtime::executor::{
    Backend, Continuation, Entry, ExecutionCall, ExecutionMemory, Outcome,
};
use crate::runtime::scheduler::{Readiness, ScheduledTimer, Task, TaskId, TimerDeadline};
use crate::runtime::time::Nanos;
use crate::runtime::{
    BindingCall, RuntimeHeap, TickResult, Worker, WorkerId, WorkerOptions, World, WorldState,
    current_runnable_scope,
};
use crate::world::RuntimeId;

/// Build one resource id owned by the primary test worker.
pub(crate) fn test_resource_id(local_id: u64) -> ResourceId {
    ResourceId::new(WorkerId(1), local_id)
}

/// Build one native binding call for runtime tests.
pub(crate) fn binding_call<'host>(
    worker: &mut Worker,
    host: &'host dyn Host,
    host_queue: &'host HostQueue,
    world: &mut WorldState,
) -> BindingCall<'host> {
    let is_process_main = host.is_process_main_context();

    BindingCall {
        runtime_id: worker.runtime_id,
        worker_id: worker.id,
        environment: worker.environment.clone(),
        options: worker.options.clone(),
        diagnostics: worker.diagnostics.clone(),
        bindings: &worker.bindings,
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
    yield v0, b1(v0)
b1(v1: int32, v2: int32):
    v3: int32 = int.add v1, v2
    yield v3, b2(v3)
b2(v4: int32, v5: int32):
    return v4
}

function test.complete(v0: int32): int32 {
b0(v0: int32):
    yield v0, b1(v0)
b1(v1: int32, v2: int32):
    return v1
}
"#;

/// VM-backed test backend builder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TestBackend {
    /// MIR text used to build the VM machine.
    mir: &'static str,
}

impl Default for TestBackend {
    fn default() -> Self {
        Self {
            mir: TEST_ENGINE_MIR,
        }
    }
}

impl TestBackend {
    /// Build one VM machine for this test backend.
    fn machine(self) -> vm::Machine {
        vm_machine_from_mir(self.mir)
    }
}

impl From<TestBackend> for Backend {
    fn from(test_backend: TestBackend) -> Self {
        Self::from(test_backend.machine())
    }
}

/// Build one VM machine from MIR text.
pub(crate) fn vm_machine_from_mir(mir: &str) -> vm::Machine {
    let (tree, strings) = Parser::parse(FileId::new(0), mir, ParseOptions::default())
        .finish()
        .expect("runtime test MIR should parse");

    vm::Machine::build_with_options(tree, strings, vm::MachineOptions::test())
        .expect("runtime test VM machine should build")
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
    constant_space: program::StaticSpace,
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
    pub(crate) fn build(options: &RuntimeOptions, backend: impl Into<Backend>) -> Self {
        let (world, shared, constant_space, shared_static, worker) =
            worker_for_options(options, backend);

        Self {
            world,
            heap: shared,
            constant_space,
            shared_static,
            worker,
        }
    }

    /// Enqueue one native task with explicit identifiers.
    pub(crate) fn enqueue_task_native(&mut self, task_id: u64, continuation_id: u64, priority: u8) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker.event_loop.enqueue_task(Task {
            id: TaskId::new(task_id),
            runnable: continuation,
            resume_value: program::Value::Void,
            priority,
        });
    }

    /// Register one native timer waiter.
    pub(crate) fn add_timer_waiter_native(
        &mut self,
        handle: u64,
        continuation_id: u64,
        priority: u8,
    ) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker
            .add_timer_waiter(
                test_resource_id(handle),
                continuation,
                program::Value::Void,
                priority,
            )
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

    /// Register one native readable-resource waiter.
    pub(crate) fn add_resource_waiter_native(
        &mut self,
        resource_id: u64,
        continuation_id: u64,
        priority: u8,
    ) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker
            .add_resource_waiter(
                test_resource_id(resource_id),
                Readiness::Readable,
                continuation,
                program::Value::Void,
                priority,
            )
            .expect("resource waiter should register");
    }

    /// Register one native host waiter.
    pub(crate) fn add_host_waiter_native(
        &mut self,
        kind: HostEventKind,
        continuation_id: u64,
        priority: u8,
    ) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker
            .add_host_waiter(kind, continuation, program::Value::Void, priority)
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
        self.worker
            .tick(
                &mut self.world.state,
                &self.heap,
                &mut self.shared_static,
                &self.constant_space,
                self.world.host.as_ref(),
                &self.world.host_queue,
            )
            .expect("tick should execute runtime work")
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
            Some(TaskId::new(task_id)),
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
    pub(crate) fn build(options: &RuntimeOptions, backend: impl Into<Backend>) -> Self {
        let environment = Arc::new(Environment::default());
        let mut world = World::new(options, environment.clone()).expect("world should build");
        let runtime_id = world
            .spawn_runtime(environment, options, backend)
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

    /// Spawn one additional worker with one explicit backend and return its id.
    pub(crate) fn spawn_worker(&mut self, backend: impl Into<Backend>) -> WorkerId {
        self.world
            .spawn_worker(self.runtime_id, WorkerOptions::default(), backend)
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

    /// Execute one runtime tick and fail loudly on runtime errors.
    pub(crate) fn tick(&mut self) -> TickResult {
        self.world.tick().expect("runtime tick should succeed")
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
                        "test.complete",
                        value,
                    )
                },
            )
            .expect("worker should exist in runtime")
    }
}

/// Build runtime-owned shared heap state for one test world.
pub(crate) fn runtime_shared_heap(world: &World, options: &RuntimeOptions) -> RuntimeHeap {
    RuntimeHeap::new(
        world.memory.allocator.clone(),
        world.memory.shared_collector.clone(),
        options,
        Arc::new(TraceTable::new()),
    )
    .expect("runtime shared heap should build")
}

/// Build one worker configured for runtime tests.
fn worker_for_options(
    options: &RuntimeOptions,
    backend: impl Into<Backend>,
) -> (
    World,
    RuntimeHeap,
    program::StaticSpace,
    program::StaticSpace,
    Worker,
) {
    let mut world =
        World::new(options, Environment::default()).expect("runtime test world should build");

    // construct one runtime worker from explicit options
    let shared = runtime_shared_heap(&world, options);
    let world_state = &mut world.state;

    let backend = backend.into();
    let constant_space = backend.constants();
    let mut shared_static = backend.shared_statics();
    let mut worker = Worker::new_in_world(
        destack_repository::Environment::default(),
        options,
        world_state,
        &shared,
        &mut shared_static,
        &constant_space,
        WorkerOptions::default(),
        backend,
    )
    .expect("runtime test worker should build");

    // apply runtime options to binding policy state
    worker.bindings.apply_runtime_defaults(options);

    // drain initial host bootstrap events for deterministic scheduler tests
    poll_host_events(world.host.as_ref(), &world.host_queue, Some(0))
        .expect("host bootstrap events should drain");

    (world, shared, constant_space, shared_static, worker)
}

/// Start one VM continuation in a worker test harness.
pub(crate) fn start_worker_continuation(
    worker: &mut Worker,
    host: &dyn Host,
    host_queue: &HostQueue,
    world: &mut WorldState,
    runtime_heap: &RuntimeHeap,
    shared_static: &mut program::StaticSpace,
    constant_space: &program::StaticSpace,
    entry: &str,
    value: i32,
) -> Continuation {
    let mut call_context = binding_call(worker, host, host_queue, world);
    let Worker {
        heap: worker_heap,
        local_static,
        executor,
        shared_cache,
        shared_gc_worker,
        ..
    } = worker;
    let context = ExecutionCall {
        host: std::ptr::NonNull::from(&mut call_context).cast(),
        memory: ExecutionMemory {
            heap: worker_heap,
            shared_heap: runtime_heap.shared.as_ref(),
            shared_cache,
            shared_gc_worker,
            local_static,
            shared_static,
            constant_space,
        },
    };
    let args = [program::Value::int32(value)];
    let outcome = executor
        .run(context, &Entry::new(entry), &args)
        .expect("test continuation should start");

    match outcome {
        Outcome::Yielded { continuation, .. } => continuation,
        Outcome::Completed { .. } => panic!("test continuation entry should yield"),
    }
}
