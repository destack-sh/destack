use std::sync::Arc;

use destack_artifact::{ConditionSet, Host, Platform, Runtime};
use destack_heap as heap;
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};

use crate::binding::BindingTable;
use crate::diagnostic::RuntimeResult;
use crate::host::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::host::time::TimerClock;
use crate::host::{
    HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId,
};
use crate::machine::{Engine, Entry};
use crate::runtime::SharedHeap;
use crate::worker::scheduler::{
    Callback, Invocation, Readiness, RunnableId, ScheduledTimer, TimerDeadline,
};
use crate::worker::{Worker, WorkerRunOutcome};
use crate::world::time::Nanos;
use crate::world::{Entity, EntityKind, World};

/// One isolated worker fixture.
#[derive(Debug)]
pub(crate) struct TestWorker {
    /// The world that owns the worker lifetime.
    world: World,
    /// The worker under test.
    worker: Worker,
    /// Runtime-owned shared heap state used by the worker.
    heap: Arc<SharedHeap>,
    /// Immutable program constant space used by the worker.
    constant_space: program::StaticImage,
    /// Runtime-owned shared static bytes used by the worker.
    shared_static: program::StaticSpace,
}

impl TestWorker {
    /// Build one test worker.
    pub(crate) fn build(
        options: &RuntimeOptions,
        program: program::Program,
        bindings: BindingTable,
        engine: Engine,
    ) -> Self {
        let mut world =
            World::new(options, Environment::default()).expect("runtime test world should build");

        // build program and runtime-owned storage
        let program = Arc::new(program);
        let shared = SharedHeap::new(
            world.memory.clone(),
            world.shared_collector.clone(),
            options,
            &program,
        )
        .expect("runtime shared heap should build");
        let constant_space = *program.constants();
        let shared_static = program
            .materialize_shared_statics(world.memory.clone())
            .expect("shared test statics should build");

        // allocate the isolated runtime and worker identities
        let runtime_id = world
            .state
            .allocate_runtime_id()
            .expect("test runtime identity should allocate");
        let worker_id = world
            .state
            .allocate_worker_id()
            .expect("test worker identity should allocate");
        let runtime_entity = Entity::new(runtime_id.entity_id(), EntityKind::RUNTIME);
        let worker_entity = Entity::new(worker_id.entity_id(), EntityKind::WORKER);
        world
            .state
            .register_runtime_topology(runtime_id, runtime_entity)
            .expect("test runtime topology should register");
        world
            .state
            .register_worker_topology(runtime_id, worker_id, worker_entity)
            .expect("test worker topology should register");

        // create the worker over the runtime storage
        let worker = Worker::new(
            Arc::new(Environment::default()),
            Arc::new(options.clone()),
            Self::conditions(),
            &mut world.state,
            &shared,
            runtime_id,
            worker_id,
            program,
            Arc::new(bindings),
            &engine,
        )
        .expect("runtime test worker should build");

        // drain initial host bootstrap events for deterministic scheduler tests
        world
            .host_queue
            .poll(world.host.as_ref(), Some(0))
            .expect("host bootstrap events should drain");

        Self {
            world,
            heap: shared,
            constant_space,
            shared_static,
            worker,
        }
    }

    /// Build the conditions used by runtime tests.
    pub(crate) fn conditions() -> Arc<ConditionSet> {
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

    /// Enqueue one task and return its execution identifier.
    pub(crate) fn enqueue_task(&mut self, entry: &str, value: u64) -> RunnableId {
        let invocation = self.invocation(entry, value);

        self.worker.event_loop.enqueue_task(invocation)
    }

    /// Run one entrypoint and return its exact Program value.
    pub(crate) fn run_entrypoint(
        &mut self,
        entry: &str,
        value: u64,
    ) -> RuntimeResult<program::Value> {
        let (_, value) = self.call(entry, value);

        self.worker.run_entrypoint(
            &mut self.world.state,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
            &Entry::new(entry),
            &[value],
        )
    }

    /// Register one timer waiter.
    pub(crate) fn add_timer_waiter(&mut self, entry: &str, handle: u64, value: u64) {
        let callback = self.callback(entry, value);

        self.worker
            .add_timer_waiter(self.resource(handle), callback);
    }

    /// Remove one timer waiter and return whether one waiter was present.
    pub(crate) fn remove_timer_waiter(&mut self, handle: u64) -> bool {
        self.worker
            .remove_timer_waiter(self.resource(handle))
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
                resource_id: self.resource(handle),
                deadline: TimerDeadline {
                    clock,
                    at: Nanos::new(fire_at_nanos),
                },
                interval: interval_nanos.map(Nanos::new),
            })
            .expect("timer should schedule");
    }

    /// Register one readable-resource waiter.
    pub(crate) fn add_resource_waiter(&mut self, entry: &str, handle: u64, value: u64) {
        let callback = self.callback(entry, value);

        self.worker
            .add_resource_waiter(self.resource(handle), Readiness::Readable, callback);
    }

    /// Register one host waiter.
    pub(crate) fn add_host_waiter(&mut self, entry: &str, kind: HostEventKind, value: u64) {
        let callback = self.callback(entry, value);

        self.worker.add_host_waiter(kind, callback);
    }

    /// Enqueue one synthetic I/O wake for dispatch tests.
    pub(crate) fn enqueue_io_event(&mut self, handle: u64, token: u64, data: u64) {
        self.worker
            .event_loop
            .enqueue_poller_wakes(vec![PollerEvent {
                resource_id: self.resource(handle),
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

    /// Run once and fail loudly on runtime errors.
    pub(crate) fn run(&mut self) -> bool {
        let outcome = self.run_task().expect("runtime work should execute");

        matches!(outcome, WorkerRunOutcome::Progressed { .. })
    }

    /// Run one queued task.
    pub(crate) fn run_task(&mut self) -> RuntimeResult<WorkerRunOutcome> {
        self.worker.run_task(
            &mut self.world.state,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
        )
    }

    /// Run one queued microtask.
    pub(crate) fn run_microtask(&mut self) -> RuntimeResult<WorkerRunOutcome> {
        self.worker.run_microtask(
            &mut self.world.state,
            &self.heap,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
        )
    }

    /// Run until idle and fail loudly on runtime errors.
    pub(crate) fn run_until_idle(&mut self) {
        while self.run() {}
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

    /// Return whether the event loop has pending work.
    pub(crate) fn has_pending_work(&self) -> bool {
        self.worker.event_loop.has_pending_work()
    }

    /// Build one direct invocation in the wrapped worker.
    fn invocation(&self, entry: &str, value: u64) -> Invocation {
        let (function, value) = self.call(entry, value);

        Invocation::call(function, [value])
    }

    /// Build one repeatable callback in the wrapped worker.
    fn callback(&self, entry: &str, value: u64) -> Callback {
        let (function, value) = self.call(entry, value);

        Callback::call(function, [value])
    }

    /// Resolve one test function and its single argument.
    fn call(&self, entry: &str, value: u64) -> (program::FunctionId, program::Value) {
        let value = i32::try_from(value).expect("test value should fit int32");
        let function = self
            .worker
            .program
            .function_id_by_name(entry)
            .expect("runtime test function should exist");
        let parameters = self
            .worker
            .program
            .function_parameters(function)
            .expect("runtime test function should have a signature");
        let ty = *parameters
            .first()
            .expect("runtime test invocation should accept one argument");
        let value = self
            .worker
            .program
            .value(ty, [program::Word::int32(value)])
            .expect("runtime test argument should match its function parameter");

        (function, value)
    }

    /// Build one resource id owned by the wrapped worker.
    fn resource(&self, local_id: u64) -> ResourceId {
        ResourceId::new(self.worker.id, local_id)
    }
}
