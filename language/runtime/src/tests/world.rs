use std::sync::Arc;

use destack_artifact::ConditionSet;
use destack_core::CaptureMode;
use destack_heap as heap;
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};
use destack_vm as vm;

use crate::binding::BindingTable;
use crate::host::time::TimerClock;
use crate::host::{HostEvent, HostEventKind, ResourceId};
use crate::machine::Engine;
use crate::tests::{TestProgram, TestWorker};
use crate::worker::scheduler::{Callback, Invocation, RunnableId, ScheduledTimer, TimerDeadline};
use crate::worker::{Worker, WorkerId, WorkerOptions};
use crate::world::time::Nanos;
use crate::world::{
    CheckpointId, Moment, RestoreContext, Run, RunOutcome, RuntimeId, Stop, World, WorldSnapshot,
};

/// One world fixture with a selected runtime.
#[derive(Debug)]
pub(crate) struct TestWorld {
    /// The world under test.
    world: World,
    /// The selected runtime.
    runtime_id: RuntimeId,
}

impl TestWorld {
    /// Build one test world.
    pub(crate) fn build(options: &RuntimeOptions, program: TestProgram) -> Self {
        let environment = Arc::new(Environment::default());
        let mut world = World::new(options, environment.clone()).expect("world should build");
        let program = Arc::new(program.build());
        let engine = Engine::new(program, vm::MachineLimits::test());
        let runtime_id = world
            .spawn_runtime(
                environment,
                options,
                TestWorker::conditions(),
                Arc::new(BindingTable::new()),
                engine,
            )
            .expect("runtime should spawn");

        world
            .host_queue
            .poll(world.host.as_ref(), Some(0))
            .expect("host bootstrap events should drain");

        Self { world, runtime_id }
    }

    /// Restore one test world from serialized snapshot bytes.
    pub(crate) fn from_snapshot_bytes(bytes: &[u8], runtime_id: RuntimeId) -> Self {
        let world = World::from_snapshot_bytes(bytes, RestoreContext::empty())
            .expect("world snapshot should restore");
        world
            .runtime(runtime_id)
            .expect("restored runtime should exist");

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

    /// Select one runtime for fixture operations.
    pub(crate) fn select_runtime(&mut self, runtime_id: RuntimeId) {
        self.world
            .runtime(runtime_id)
            .expect("selected runtime should exist");
        self.runtime_id = runtime_id;
    }

    /// Borrow the wrapped world.
    pub(crate) fn world(&self) -> &World {
        &self.world
    }

    /// Build the first argument accepted by one function.
    pub(crate) fn argument(
        &self,
        entry: &str,
        words: impl IntoIterator<Item = program::Word>,
    ) -> program::Value {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let function = runtime
            .program
            .function_id_by_name(entry)
            .expect("runtime test function should exist");
        let parameters = runtime
            .program
            .function_parameters(function)
            .expect("runtime test function should have a signature");
        let ty = *parameters
            .first()
            .expect("runtime test function should accept one argument");

        runtime
            .program
            .value(ty, words)
            .expect("runtime test argument should match its function parameter")
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

    /// Spawn one additional runtime and return its id.
    pub(crate) fn spawn_runtime(
        &mut self,
        options: &RuntimeOptions,
        program: TestProgram,
    ) -> RuntimeId {
        let environment = Arc::new(Environment::default());
        let program = Arc::new(program.build());
        let engine = Engine::new(program, vm::MachineLimits::test());

        self.world
            .spawn_runtime(
                environment,
                options,
                TestWorker::conditions(),
                Arc::new(BindingTable::new()),
                engine,
            )
            .expect("runtime should spawn")
    }

    /// Return the conditions shared by this runtime.
    pub(crate) fn conditions(&self) -> Arc<ConditionSet> {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .conditions
            .clone()
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

    /// Run one world microtask and fail loudly on runtime errors.
    pub(crate) fn run_microtask(&mut self) -> RunOutcome {
        self.world
            .run(Run::Microtask)
            .expect("world microtask run should succeed")
    }

    /// Run one world task and require an execution stop.
    pub(crate) fn run_to_stop(&mut self) -> Stop {
        let outcome = self.run_task();
        let RunOutcome::Stopped { stop } = outcome else {
            panic!("world task should stop");
        };

        stop
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
        let worker_id = self.default_worker_id();
        let reference = self.allocate_cached_shared(worker_id, shape);
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        let heap = runtime.heap.clone();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("worker should exist in runtime");

        heap.shared.flush_allocation_cache(&mut worker.shared_cache);

        reference
    }

    /// Allocate one shared block into a worker-local allocation cache.
    pub(crate) fn allocate_cached_shared(
        &mut self,
        worker_id: WorkerId,
        shape: heap::AllocationShape,
    ) -> heap::SharedHeapReference {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        let heap = runtime.heap.clone();
        let program = runtime.program.clone();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("worker should exist in runtime");
        let plan = heap.shared.options().allocation_plan(&shape);

        heap.shared
            .allocate_zeroed(
                &worker.shared_mark_worker,
                &mut worker.shared_cache,
                plan,
                &shape.trace_map,
                program.trace_view(),
            )
            .expect("shared test allocation should succeed")
    }

    /// Request one shared collection cycle.
    pub(crate) fn request_shared_gc(&mut self) {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");

        runtime.heap.shared.request_gc();
    }

    /// Start one requested shared collection cycle.
    pub(crate) fn start_shared_gc(&mut self) {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");

        runtime.heap.shared.request_gc();
        runtime
            .advance_shared_gc()
            .expect("shared collection should start");
    }

    /// Return the number of shared allocations published by workers.
    pub(crate) fn shared_allocation_count(&self) -> usize {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .heap
            .shared
            .usage()
            .allocation_count
    }

    /// Capture one runtime image.
    pub(crate) fn capture(&mut self, mode: CaptureMode) {
        self.world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist")
            .capture_image(mode)
            .expect("runtime image should capture");
    }

    /// Queue one worker for shared direct-root publication.
    pub(crate) fn queue_shared_roots(&mut self, worker_id: WorkerId) {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .heap
            .queue_root_scan(worker_id);
    }

    /// Return whether one worker owes shared direct-root publication.
    pub(crate) fn has_pending_shared_roots(&self, worker_id: WorkerId) -> bool {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .heap
            .roots()
            .pending_root_epoch(worker_id)
            .is_some()
    }

    /// Return the shared roots published for the active collection.
    pub(crate) fn shared_roots(&self) -> Arc<[heap::SharedHeapReference]> {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .heap
            .roots()
            .roots_snapshot()
    }

    /// Run one idle worker safepoint.
    pub(crate) fn run_safepoint(&mut self) -> Option<(WorkerId, heap::GcAdvance)> {
        let world = &mut self.world;
        let runtime = world
            .runtimes
            .get_mut(&self.runtime_id)
            .expect("runtime should exist");

        runtime
            .run_safepoint(&mut world.state, world.host.as_ref(), &world.host_queue)
            .expect("runtime safepoint should succeed")
    }

    /// Deliver one host event to matching worker waiters.
    pub(crate) fn deliver_host_event(&mut self, event: HostEvent) -> bool {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        let is_marking = runtime.heap.is_marking();

        runtime
            .deliver_host_event(event, is_marking)
            .expect("host event should deliver")
    }

    /// Register one host waiter on one explicit worker.
    pub(crate) fn add_host_waiter(
        &mut self,
        worker_id: WorkerId,
        entry: &str,
        kind: HostEventKind,
        value: program::Value,
    ) {
        let function = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .program
            .function_id_by_name(entry)
            .expect("runtime test function should exist");
        let callback = Callback::call(function, [value]);

        self.with_worker_mut(worker_id, |worker| {
            worker.add_host_waiter(kind, callback);
        });
    }

    /// Return whether one shared test allocation is live.
    pub(crate) fn is_shared_live(&self, reference: heap::SharedHeapReference) -> bool {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");

        runtime.heap.shared.is_heap_live(reference)
    }

    /// Read one shared heap allocation range.
    pub(crate) fn read_shared(
        &self,
        reference: heap::SharedHeapReference,
        byte_len: usize,
    ) -> Vec<u8> {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let heap_offset =
            runtime.heap.shared.heap_base_address() - self.world.memory.base_address();
        let offset = heap_offset + reference.offset();

        self.world
            .memory
            .read_bytes(offset, byte_len)
            .expect("shared test allocation should read")
    }

    /// Write one shared heap allocation range.
    pub(crate) fn write_shared(&mut self, reference: heap::SharedHeapReference, bytes: &[u8]) {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let heap_offset =
            runtime.heap.shared.heap_base_address() - self.world.memory.base_address();
        let offset = heap_offset + reference.offset();

        runtime
            .heap
            .shared
            .write_barrier_bytes(reference, 0, bytes, runtime.program.trace_view())
            .expect("shared test write barrier should succeed");
        self.world
            .memory
            .write_bytes(offset, bytes)
            .expect("shared test allocation should write");
    }

    /// Create one named world checkpoint.
    pub(crate) fn checkpoint(&mut self, name: &str) -> CheckpointId {
        self.world
            .checkpoint(name)
            .expect("world checkpoint should capture")
    }

    /// Create one exact snapshot for a checkpoint.
    pub(crate) fn snapshot(&self, checkpoint: CheckpointId) -> WorldSnapshot {
        self.world
            .snapshot_checkpoint(checkpoint)
            .expect("world checkpoint should serialize")
    }

    /// Restore one exact world snapshot.
    pub(crate) fn restore(&mut self, snapshot: &WorldSnapshot) {
        self.world
            .restore_snapshot(snapshot, RestoreContext::empty())
            .expect("world snapshot should restore");
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

    /// Schedule one timer on one explicit worker.
    pub(crate) fn schedule_timer(
        &mut self,
        worker_id: WorkerId,
        clock: TimerClock,
        handle: u64,
        fire_at_nanos: u64,
        interval_nanos: Option<u64>,
    ) {
        self.with_worker_mut(worker_id, |worker| {
            worker
                .event_loop
                .schedule_timer(ScheduledTimer {
                    resource_id: ResourceId::new(worker.worker_id(), handle),
                    deadline: TimerDeadline {
                        clock,
                        at: Nanos::new(fire_at_nanos),
                    },
                    interval: interval_nanos.map(Nanos::new),
                })
                .expect("timer should schedule");
        });
    }

    /// Return one executable point by function name and operation index.
    pub(crate) fn point(&self, entry: &str, operation: u32) -> program::ProgramPoint {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let function = runtime
            .program
            .function_id_by_name(entry)
            .expect("runtime test function should exist");

        program::ProgramPoint::new(function, operation)
    }

    /// Enqueue one direct invocation in one explicit worker.
    pub(crate) fn enqueue_task(
        &mut self,
        worker_id: WorkerId,
        entry: &str,
        value: u64,
    ) -> RunnableId {
        let invocation = self.invocation(worker_id, entry, value);
        let runtime = self
            .world
            .runtimes
            .get_mut(&self.runtime_id)
            .expect("runtime should exist");
        let worker = runtime
            .worker_mut(worker_id)
            .expect("worker should exist in runtime");

        worker.event_loop.enqueue_task(invocation)
    }

    /// Enqueue one microtask in one explicit worker.
    pub(crate) fn enqueue_microtask(
        &mut self,
        worker_id: WorkerId,
        entry: &str,
        value: u64,
    ) -> RunnableId {
        let invocation = self.invocation(worker_id, entry, value);
        let runtime = self
            .world
            .runtimes
            .get_mut(&self.runtime_id)
            .expect("runtime should exist");
        let worker = runtime
            .worker_mut(worker_id)
            .expect("worker should exist in runtime");

        worker.event_loop.enqueue_microtask(invocation)
    }

    /// Build one direct invocation for one explicit worker.
    pub(crate) fn invocation(
        &mut self,
        worker_id: WorkerId,
        entry: &str,
        value: u64,
    ) -> Invocation {
        let (function, value) = self.call(worker_id, entry, value);

        Invocation::call(function, [value])
    }

    /// Build one repeatable callback for one explicit worker.
    pub(crate) fn callback(&mut self, worker_id: WorkerId, entry: &str, value: u64) -> Callback {
        let (function, value) = self.call(worker_id, entry, value);

        Callback::call(function, [value])
    }

    /// Resolve one test function and its single argument.
    fn call(
        &self,
        worker_id: WorkerId,
        entry: &str,
        value: u64,
    ) -> (program::FunctionId, program::Value) {
        let value = i32::try_from(value).expect("test value should fit int32");
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let function = runtime
            .worker(worker_id)
            .expect("worker should exist in runtime")
            .program
            .function_id_by_name(entry)
            .expect("runtime test function should exist");
        let parameters = runtime
            .program
            .function_parameters(function)
            .expect("runtime test function should have a signature");
        let ty = *parameters
            .first()
            .expect("runtime test invocation should accept one argument");
        let value = runtime
            .program
            .value(ty, [program::Word::int32(value)])
            .expect("runtime test argument should match its function parameter");

        (function, value)
    }
}
