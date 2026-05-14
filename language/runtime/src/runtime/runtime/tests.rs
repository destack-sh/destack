use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;
use destack_workspace::{RuntimeOptions, SchedulerOptions};
use {destack_engine as engine, destack_vm as vm};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingEngine;
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::time::{HostClockSource, TimerClock};
use crate::host::{
    HostEvent, HostEventKind, HostSession, LifecycleEvent, LifecycleSourceKind, LifecycleState,
    ResourceId, default_compile_target_host,
};
use crate::runtime::engine::{CallContext, Continuation, Engine, Entry, MemoryContext, Outcome};
use crate::runtime::scheduler::{
    Microtask, MicrotaskId, Readiness, ScheduledTimer, Task, TaskId, TimerDeadline,
};
use crate::runtime::time::Nanos;
use crate::runtime::{
    BindingCallContext, ExecutionContext, RuntimeId, SharedHeap, TickResult, Worker, WorkerId,
    WorkerOptions, World, WorldState, current_runnable_scope,
};

/// Test host clock source for deterministic host-time runtime tests.
#[derive(Debug, Default)]
pub(crate) struct TestHostClockSource {
    /// Current test wall time in nanoseconds.
    wall_nanos: AtomicU64,
    /// Current test monotonic time in nanoseconds.
    mono_nanos: AtomicU64,
}

impl TestHostClockSource {
    /// Create one test host clock source.
    pub(crate) fn new(wall_nanos: u64, mono_nanos: u64) -> Self {
        Self {
            wall_nanos: AtomicU64::new(wall_nanos),
            mono_nanos: AtomicU64::new(mono_nanos),
        }
    }

    /// Advance wall and monotonic time together by one duration.
    pub(crate) fn advance_both_nanos(&self, delta_nanos: u64) {
        let _ = self.wall_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
        let _ = self.mono_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
    }

    /// Advance only monotonic time by one duration.
    pub(crate) fn advance_mono_nanos(&self, delta_nanos: u64) {
        let _ = self.mono_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
    }

    /// Jump wall time by one signed delta.
    pub(crate) fn jump_wall_nanos(&self, delta_nanos: i64) {
        let current = self.wall_nanos();
        let next = current.saturating_add_signed(delta_nanos);
        self.wall_nanos.store(next, Ordering::Relaxed);
    }

    /// Return one test wall time sample.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.wall_nanos.load(Ordering::Relaxed)
    }

    /// Return one test monotonic time sample.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.mono_nanos.load(Ordering::Relaxed)
    }
}

/// Build one resource id owned by the primary test worker.
pub(crate) fn test_resource_id(local_id: u64) -> ResourceId {
    ResourceId::new(WorkerId(1), local_id)
}

/// Build one native binding call context for runtime tests.
pub(crate) fn binding_call_context(
    worker: &mut Worker,
    host: &HostSession,
    world: &mut WorldState,
) -> BindingCallContext {
    let execution_context = ExecutionContext::new(host.is_process_main_context());

    BindingCallContext {
        worker: worker as *mut Worker,
        event_loop: worker.event_loop.as_ref() as *const _,
        host: host as *const HostSession,
        world: world as *mut WorldState,
        engine: BindingEngine::Native,
        scope: current_runnable_scope(),
        execution_context,
    }
}

impl HostClockSource for TestHostClockSource {
    /// Return one test wall-clock sample.
    fn wall_nanos(&self) -> u64 {
        self.wall_nanos()
    }

    /// Return one test monotonic-clock sample.
    fn mono_nanos(&self) -> u64 {
        self.mono_nanos()
    }

    /// Sleep by advancing test wall and monotonic time.
    fn sleep_nanos(&self, duration_nanos: u64) {
        self.advance_both_nanos(duration_nanos);
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

/// VM-backed test engine builder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TestEngine {
    /// MIR text used to build the VM engine.
    mir: &'static str,
}

impl Default for TestEngine {
    fn default() -> Self {
        Self {
            mir: TEST_ENGINE_MIR,
        }
    }
}

impl TestEngine {
    /// Build one VM isolate for this test engine.
    fn isolate(self) -> vm::Isolate {
        vm_engine_from_mir(self.mir)
    }
}

impl From<TestEngine> for Engine {
    fn from(engine: TestEngine) -> Self {
        Self::from(engine.isolate())
    }
}

/// Build one VM isolate from MIR text.
pub(crate) fn vm_engine_from_mir(mir: &str) -> vm::Isolate {
    let (tree, strings) = Parser::parse(FileId::new(0), mir, ParseOptions::default())
        .finish()
        .expect("runtime test MIR should parse");

    vm::Isolate::build_with_options(
        vm::IsolateId::new(1),
        tree,
        strings,
        vm::IsolateOptions::test(),
    )
    .expect("runtime test VM engine should build")
}

/// Test harness for worker scheduling tests.
#[derive(Debug)]
pub(crate) struct TestRuntime {
    /// Test world that owns the worker lifetime.
    world: World,
    /// Optional host clock source used by blocking test poller waits.
    host_clock_source: Option<Arc<dyn HostClockSource>>,
    /// Wrapped worker under test.
    worker: Worker,
    /// Runtime-owned shared heap state used by the worker.
    shared: SharedHeap,
    /// Runtime-owned static bytes used by the worker.
    runtime_static: engine::StaticSpace,
    /// Wrapped host session under test.
    host: HostSession,
}

/// Test harness for multi-worker runtime scheduler tests.
#[derive(Debug)]
pub(crate) struct TestWorldRuntime {
    /// Test world that owns the runtime lifetime.
    world: World,
    /// Wrapped runtime identity under test.
    runtime_id: RuntimeId,
}

/// Test poller for runtime ingress tests.
#[derive(Debug, Default)]
pub(crate) struct TestPoller {
    /// Events returned by the next poll.
    events: Vec<PollerEvent>,
    /// Optional clock source used to model blocking waits.
    host_clock_source: Option<Arc<dyn HostClockSource>>,
}

impl TestPoller {
    /// Build one test poller from explicit events.
    pub(crate) fn with_events(events: Vec<PollerEvent>) -> Self {
        Self {
            events,
            host_clock_source: None,
        }
    }

    /// Build one test poller with one host clock source.
    fn with_host_clock_source(host_clock_source: Option<Arc<dyn HostClockSource>>) -> Self {
        Self {
            events: Vec::new(),
            host_clock_source,
        }
    }
}

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

    /// Return the test poll result once and then drain it.
    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        if self.events.is_empty()
            && let Some(timeout_nanos) = timeout_nanos
            && let Some(host_clock_source) = &self.host_clock_source
        {
            host_clock_source.sleep_nanos(timeout_nanos);
        }

        Ok(std::mem::take(&mut self.events))
    }
}

impl TestWorldRuntime {
    /// Borrow the wrapped world.
    pub(crate) fn world(&self) -> &World {
        &self.world
    }
}

impl TestRuntime {
    /// Create one test worker with default options.
    pub(crate) fn new() -> Self {
        let (world, shared, runtime_static, worker, host) =
            worker_for_options_with_engine(&RuntimeOptions::default(), TestEngine::default());

        Self {
            world,
            host_clock_source: None,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Create one test worker with explicit options and one explicit engine.
    pub(crate) fn with_options_and_engine(
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> Self {
        let (world, shared, runtime_static, worker, host) =
            worker_for_options_with_engine(options, engine);

        Self {
            world,
            host_clock_source: None,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Create one test worker with explicit options, one explicit engine, and one host clock source.
    pub(crate) fn with_options_engine_and_host_clock_source(
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let host_clock_source = Some(host_clock_source);
        let (world, shared, runtime_static, worker, host) =
            worker_for_options_with_engine_and_host_clock_source(
                options,
                engine,
                host_clock_source.clone(),
            );

        Self {
            world,
            host_clock_source,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Enqueue one native task with explicit identifiers.
    pub(crate) fn enqueue_task_native(&mut self, task_id: u64, continuation_id: u64, priority: u8) {
        let continuation = self.yielding_continuation(continuation_id);

        self.worker.event_loop.enqueue_task(Task {
            id: TaskId::new(task_id),
            runnable: continuation,
            resume_value: engine::Value::Void,
            priority,
        });
    }

    /// Enqueue one native microtask with explicit identifiers.
    pub(crate) fn enqueue_microtask_native(&mut self, microtask_id: u64, continuation_id: u64) {
        let continuation = self.completing_continuation(continuation_id);

        self.worker.event_loop.enqueue_microtask(Microtask {
            id: MicrotaskId::new(microtask_id),
            continuation,
            resume_value: engine::Value::Void,
        });
    }

    /// Configure scheduler options and fail loudly in tests.
    pub(crate) fn configure_scheduler(&mut self, options: SchedulerOptions) {
        self.worker
            .event_loop
            .configure(options)
            .expect("scheduler options should configure");
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
                engine::Value::Void,
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
                engine::Value::Void,
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
            .add_host_waiter(kind, continuation, engine::Value::Void, priority)
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
                &self.shared,
                &self.runtime_static,
                &self.host,
            )
            .expect("tick should execute runtime work")
    }

    /// Tick until idle and fail loudly on runtime errors.
    pub(crate) fn tick_until_idle(&mut self) {
        let mut poller = TestPoller::with_host_clock_source(self.host_clock_source.clone());
        self.worker
            .run_event_loop(
                &mut self.world.state,
                &self.shared,
                &self.runtime_static,
                &self.host,
                None,
                None,
                &mut poller,
            )
            .expect("tick until idle should complete");
    }

    /// Run until one task completes.
    pub(crate) fn run_loop_until_task_complete(
        &mut self,
        task_id: u64,
    ) -> RuntimeResult<engine::Value> {
        let mut poller = TestPoller::with_host_clock_source(self.host_clock_source.clone());

        let output = self.worker.run_event_loop(
            &mut self.world.state,
            &self.shared,
            &self.runtime_static,
            &self.host,
            Some(TaskId::new(task_id)),
            None,
            &mut poller,
        )?;
        let Some(output) = output else {
            return Err(RuntimeError::EventLoopIdle { task_id }.boxed());
        };

        Ok(output)
    }

    /// Run until one task completes or one timeout elapses.
    pub(crate) fn run_loop_until_task_complete_with_timeout(
        &mut self,
        task_id: u64,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<engine::Value>> {
        let mut poller = TestPoller::with_host_clock_source(self.host_clock_source.clone());

        self.worker.run_event_loop(
            &mut self.world.state,
            &self.shared,
            &self.runtime_static,
            &self.host,
            Some(TaskId::new(task_id)),
            timeout_nanos,
            &mut poller,
        )
    }

    /// Return whether the event loop has pending work.
    pub(crate) fn has_pending_work(&self) -> bool {
        self.worker.event_loop.has_pending_work()
    }

    /// Return whether the event loop has pending microtasks.
    pub(crate) fn has_microtasks(&self) -> bool {
        self.worker.event_loop.has_microtasks()
    }

    /// Create one VM continuation that yields once when scheduled.
    pub(crate) fn yielding_continuation(&mut self, value: u64) -> Continuation {
        let value = i32::try_from(value).expect("test continuation id should fit int32");

        self.start_continuation("test.task", value)
    }

    /// Create one VM continuation that completes when scheduled.
    pub(crate) fn completing_continuation(&mut self, value: u64) -> Continuation {
        let value = i32::try_from(value).expect("test continuation id should fit int32");

        self.start_continuation("test.complete", value)
    }

    /// Start one yielding VM function and return its continuation.
    fn start_continuation(&mut self, entry: &str, value: i32) -> Continuation {
        start_worker_continuation(
            &mut self.worker,
            &self.host,
            &mut self.world.state,
            &self.shared,
            &self.runtime_static,
            entry,
            value,
        )
    }

    /// Return current runtime wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Return current runtime monotonic time in nanoseconds.
    pub(crate) fn mono_nanos(&self) -> u64 {
        self.world.mono_nanos()
    }
}

impl TestWorldRuntime {
    /// Create one runtime with explicit options and one explicit engine.
    pub(crate) fn with_options_and_engine(
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> Self {
        let mut world = World::from_options(options).expect("world should build");
        let runtime_id = world
            .spawn_runtime(Vec::new(), options, engine)
            .expect("runtime should spawn");
        world
            .runtime(runtime_id)
            .expect("runtime should exist")
            .host()
            .poll(Some(0))
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

    /// Spawn one additional worker with one explicit engine and return its id.
    pub(crate) fn spawn_worker(&mut self, engine: impl Into<Engine>) -> WorkerId {
        let options = RuntimeOptions::default();

        self.world
            .spawn_worker(self.runtime_id, &options, WorkerOptions::default(), engine)
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

    /// Attach one explicit test poller.
    pub(crate) fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        runtime.set_poller(poller);
    }

    /// Return current world wall time in nanoseconds.
    pub(crate) fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Borrow the wrapped world mutably.
    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.world
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
            state, runtimes, ..
        } = &mut self.world;
        let runtime = runtimes
            .get_mut(&self.runtime_id)
            .map(Box::as_mut)
            .expect("runtime should exist");

        runtime
            .with_worker_context(worker_id, |host, shared, runtime_static, worker| {
                start_worker_continuation(
                    worker,
                    host,
                    state,
                    shared,
                    runtime_static,
                    "test.complete",
                    value,
                )
            })
            .expect("worker should exist in runtime")
    }
}

/// Build runtime-owned shared heap state for one test world.
pub(crate) fn runtime_shared_heap(world: &World, options: &RuntimeOptions) -> SharedHeap {
    let history = world.history.read();

    SharedHeap::new(history.allocator(), history.collector(), options)
        .expect("runtime shared heap should build")
}

/// Build one worker configured for runtime tests with one explicit engine.
fn worker_for_options_with_engine(
    options: &RuntimeOptions,
    engine: impl Into<Engine>,
) -> (World, SharedHeap, engine::StaticSpace, Worker, HostSession) {
    worker_for_options_with_engine_and_host_clock_source(options, engine, None)
}

/// Build one worker configured for runtime tests with one explicit engine and one optional host clock source.
fn worker_for_options_with_engine_and_host_clock_source(
    options: &RuntimeOptions,
    engine: impl Into<Engine>,
    host_clock_source: Option<Arc<dyn HostClockSource>>,
) -> (World, SharedHeap, engine::StaticSpace, Worker, HostSession) {
    let mut world = if let Some(host_clock_source) = host_clock_source.clone() {
        World::new(options, Some(host_clock_source)).expect("runtime test world should build")
    } else {
        World::from_options(options).expect("runtime test world should build")
    };

    // construct one runtime worker from explicit options
    let shared = runtime_shared_heap(&world, options);
    let world_state = &mut world.state;

    let runtime_static = engine::StaticSpace::empty();
    let mut worker = Worker::new_in_world(
        Vec::new(),
        options,
        world_state,
        &shared,
        &runtime_static,
        WorkerOptions::default(),
        engine,
    )
    .expect("runtime test worker should build");

    // configure scheduler options for deterministic tests
    worker
        .event_loop
        .configure(options.scheduler_options().clone())
        .expect("scheduler options should configure");

    // apply runtime options to binding policy state
    worker.bindings.apply_runtime_defaults(options);

    // build the host session for this test worker
    let host = HostSession::new(default_compile_target_host(), worker.runtime_id);

    // drain initial host bootstrap events for deterministic scheduler tests
    host.poll(Some(0))
        .expect("host bootstrap events should drain");

    (world, shared, runtime_static, worker, host)
}

/// Start one VM continuation in a worker test harness.
pub(crate) fn start_worker_continuation(
    worker: &mut Worker,
    host: &HostSession,
    world: &mut WorldState,
    shared: &SharedHeap,
    runtime_static: &engine::StaticSpace,
    entry: &str,
    value: i32,
) -> Continuation {
    let mut call_context = binding_call_context(worker, host, world);
    let Worker {
        heap,
        statics,
        engine,
        shared_allocator,
        shared_gc_worker,
        ..
    } = worker;
    let context = CallContext {
        runtime: std::ptr::NonNull::from(&mut call_context).cast(),
        memory: MemoryContext {
            heap,
            shared_heap: shared.heap(),
            shared_allocator,
            shared_gc_worker,
            worker_static: statics,
            runtime_static,
        },
    };
    let args = [engine::Value::int32(value)];
    let outcome = engine
        .run(context, &Entry::new(entry), &args)
        .expect("test continuation should start");

    match outcome {
        Outcome::Yielded { continuation, .. } => continuation,
        Outcome::Completed { .. } => panic!("test continuation entry should yield"),
    }
}
