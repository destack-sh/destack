use std::sync::Arc;

use destack_artifact::{ConditionSet, Host, Platform, Runtime};
use destack_heap as heap;
use destack_memory::MemoryRange;
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions, WorldOptions};
use destack_vm as vm;

use crate::binding::BindingTable;
use crate::diagnostic::RuntimeResult;
use crate::heap::SharedCollectionState;
use crate::host::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::host::time::TimerClock;
use crate::host::{
    HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId,
};
use crate::machine::native::Code;
use crate::machine::{Engine, Entry};
use crate::scheduler::{
    Callback, HostWake, Invocation, Readiness, ResourceWake, RunnableId, ScheduledTimer,
    TimerDeadline, Wake,
};
use crate::worker::{Request, Worker, WorkerRunOutcome};
use crate::world::time::Nanos;
use crate::world::{Entity, EntityKind, World};

/// One isolated worker fixture.
#[derive(Debug)]
pub(crate) struct TestWorker {
    /// The world that owns the worker lifetime.
    world: World,
    /// The worker under test.
    worker: Worker,
    /// Shared collection state used by the worker.
    collection: Arc<SharedCollectionState>,
    /// Immutable program constant space used by the worker.
    constant_space: program::StaticSpace,
    /// Runtime-owned shared static bytes used by the worker.
    shared_static: program::StaticSpace,
}

impl TestWorker {
    /// Build one bytecode test worker.
    pub(crate) fn bytecode(
        options: &RuntimeOptions,
        program: program::Program,
        bindings: BindingTable,
    ) -> Self {
        let program = Arc::new(program);
        let engine = Engine::new(program.clone(), vm::MachineLimits::test());

        Self::build(options, program, bindings, engine)
    }

    /// Build one native test worker.
    pub(crate) fn native(
        options: &RuntimeOptions,
        program: program::Program,
        bindings: BindingTable,
        code: Code,
    ) -> Self {
        let program = Arc::new(program);
        let engine = Engine::new(program.clone(), vm::MachineLimits::test()).native(code);

        Self::build(options, program, bindings, engine)
    }

    /// Build one test worker from its complete execution engine.
    fn build(
        options: &RuntimeOptions,
        program: Arc<program::Program>,
        bindings: BindingTable,
        engine: Engine,
    ) -> Self {
        let world_options = WorldOptions::default();
        let mut world = World::new(&world_options, Environment::default())
            .expect("runtime test world should build");

        // build program and runtime-owned storage
        let local_heap_options = options
            .heap
            .local_heap_options()
            .expect("local heap options should resolve");
        let shared_heap_options = options
            .heap
            .shared_heap_options()
            .expect("shared heap options should resolve");
        let shared_heap = Arc::new(
            heap::SharedHeap::new(
                world.memory.clone(),
                options.heap.shared.limits(),
                shared_heap_options,
            )
            .expect("runtime shared heap should build"),
        );
        let collection = SharedCollectionState::new(&world.collector);
        let allocation_plans = program
            .plan_allocations(&local_heap_options, shared_heap.options())
            .expect("allocation plans should build")
            .into();
        let (constant_space, shared_static) = program
            .materialize_runtime_statics(world.memory.clone())
            .expect("test runtime statics should materialize");

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
            &shared_heap,
            &allocation_plans,
            &constant_space,
            &shared_static,
            MemoryRange {
                offset: constant_space.offset(),
                byte_len: constant_space.byte_len(),
            },
            runtime_id,
            worker_id,
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
            collection,
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
            &self.collection,
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
        let wake = ResourceWake::poller(PollerEvent {
            resource_id: self.resource(handle),
            source: PollerEventSource::Io,
            mask: PollerEventMask::READABLE,
            flags: PollerEventFlags::NONE,
            token: PollerToken(token),
            payload: PollerEventPayload::Io { data },
        });

        self.worker.event_loop.enqueue_wake(Wake::Resource(wake));
    }

    /// Enqueue one synthetic lifecycle host wake for dispatch tests.
    pub(crate) fn enqueue_lifecycle_host_event(&mut self, state: LifecycleState) {
        let event = HostEvent::Lifecycle(LifecycleEvent {
            source_kind: LifecycleSourceKind::Application,
            state,
        });

        self.worker
            .event_loop
            .enqueue_wake(Wake::Host(HostWake::new(event)));
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
            &self.collection,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
        )
    }

    /// Resume one debugger-stopped runnable.
    pub(crate) fn resume(&mut self) -> RuntimeResult<WorkerRunOutcome> {
        self.worker.resume(
            &mut self.world.state,
            &self.collection,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
        )
    }

    /// Publish one process-local request to the wrapped worker.
    pub(crate) fn request(&self, request: Request) {
        self.worker.request(request);
    }

    /// Drain queued work and fail loudly on runtime errors.
    pub(crate) fn drain(&mut self) {
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
        self.worker.heap.is_live(reference)
    }

    /// Request one full worker-local collection.
    pub(crate) fn request_full_gc(&mut self) {
        self.worker.heap.request_gc();
    }

    /// Advance one idle worker GC operation.
    pub(crate) fn step_gc(&mut self) -> RuntimeResult<Option<heap::GcAdvance>> {
        self.worker.advance_gc_once(
            &mut self.world.state,
            &self.collection,
            &mut self.shared_static,
            &self.constant_space,
            self.world.host.as_ref(),
            &self.world.host_queue,
        )
    }

    /// Return whether the event loop has pending work.
    pub(crate) fn has_pending_work(&self) -> bool {
        self.worker.has_pending_work()
    }

    /// Build one direct invocation in the wrapped worker.
    fn invocation(&self, entry: &str, value: u64) -> Invocation {
        let (function, value) = self.call(entry, value);

        Invocation::call(function, [value], program::Context::empty())
    }

    /// Build one repeatable callback in the wrapped worker.
    fn callback(&self, entry: &str, value: u64) -> Callback {
        let (function, value) = self.call(entry, value);

        Callback::call(function, [value], program::Context::empty())
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
