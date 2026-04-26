use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_core::LocalStringPool;
use destack_engine::{StaticSpace, Value};
use destack_mir::{NodeTree, ReferenceMap};
use destack_workspace::{RuntimeOptions, SchedulerOptions};
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::RuntimeResult;
use crate::host::{
    HostEvent, HostEventKind, HostLifecycleEvent, HostLifecycleSourceKind, HostLifecycleState,
    Session,
};
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::bindings::BindingEngine;
use crate::runtime::engine::engine::ErasedNativeEngine;
use crate::runtime::engine::{
    Context, Continuation, Engine, Entry, NativeContinuation, NativeImage, Outcome, Output,
};
use crate::runtime::memory::RootVisitor;
use crate::runtime::poller::{
    HostPoller, HostPollerFlags, PlatformHandle, PlatformInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::runtime::scheduler::{
    Microtask, MicrotaskId, Task, TaskId, TaskStatus, Timer, TimerDeadline,
};
use crate::runtime::time::{HostClockSource, Nanos};
use crate::runtime::world::{Branch, CheckpointId, Revision, WorldEntityKindDefinition};
use crate::runtime::{
    BindingCallContext, DropCounts, RuntimeId, RuntimeSharedHeap, TickOutcome, Worker, WorkerId,
    World, WorldRef,
};

/// Scripted host clock source for deterministic host-time runtime tests.
#[derive(Debug, Default)]
pub(super) struct ScriptedHostClockSource {
    /// Current scripted wall time in nanoseconds.
    wall_nanos: AtomicU64,
    /// Current scripted monotonic time in nanoseconds.
    mono_nanos: AtomicU64,
}

impl ScriptedHostClockSource {
    /// Create one scripted host clock source.
    pub(super) fn new(wall_nanos: u64, mono_nanos: u64) -> Self {
        Self {
            wall_nanos: AtomicU64::new(wall_nanos),
            mono_nanos: AtomicU64::new(mono_nanos),
        }
    }

    /// Advance wall and monotonic time together by one duration.
    pub(super) fn advance_both_nanos(&self, delta_nanos: u64) {
        let _ = self.wall_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
        let _ = self.mono_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
    }

    /// Advance only monotonic time by one duration.
    pub(super) fn advance_mono_nanos(&self, delta_nanos: u64) {
        let _ = self.mono_nanos.fetch_add(delta_nanos, Ordering::Relaxed);
    }

    /// Jump wall time by one signed delta.
    pub(super) fn jump_wall_nanos(&self, delta_nanos: i64) {
        let current = self.wall_nanos();
        let next = current.saturating_add_signed(delta_nanos);
        self.wall_nanos.store(next, Ordering::Relaxed);
    }

    /// Return one scripted wall time sample.
    pub(super) fn wall_nanos(&self) -> u64 {
        self.wall_nanos.load(Ordering::Relaxed)
    }

    /// Return one scripted monotonic time sample.
    pub(super) fn mono_nanos(&self) -> u64 {
        self.mono_nanos.load(Ordering::Relaxed)
    }
}

/// Build one native binding call context for runtime tests.
pub(super) fn binding_call_context(
    worker: &Worker,
    host: &Session,
    world: &WorldRef,
) -> BindingCallContext {
    BindingCallContext::from_raw(
        worker as *const Worker,
        worker.event_loop.as_ref() as *const _,
        host as *const Session,
        world as *const WorldRef,
        BindingEngine::Native,
    )
}

impl HostClockSource for ScriptedHostClockSource {
    /// Return one scripted wall-clock sample.
    fn wall_nanos(&self) -> u64 {
        self.wall_nanos()
    }

    /// Return one scripted monotonic-clock sample.
    fn mono_nanos(&self) -> u64 {
        self.mono_nanos()
    }

    /// Sleep by advancing scripted wall and monotonic time.
    fn sleep_nanos(&self, duration_nanos: u64) {
        self.advance_both_nanos(duration_nanos);
    }
}

/// Test engine that yields once, then completes.
#[derive(Debug, Clone, Default)]
pub(crate) struct TestEngine {
    /// Number of resume calls executed.
    pub(super) resume_calls: usize,
}

/// Engine that allocates into the heap when it runs or resumes.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AllocatingEngine {
    /// The number of heap values to allocate into one heap allocation.
    pub(super) heap_values: usize,
    /// The number of raw bytes to allocate into one raw allocation.
    pub(super) raw_bytes: usize,
}

impl ErasedNativeEngine for TestEngine {
    /// Run one entrypoint without yielding.
    fn run(
        &mut self,
        _context: Context<'_>,
        _entry: &Entry,
        _args: &[Value],
    ) -> RuntimeResult<Outcome<Continuation>> {
        Ok(Outcome::Completed {
            output: void_output(),
        })
    }

    /// Resume one continuation and yield once before completion.
    fn resume(
        &mut self,
        _context: Context<'_>,
        _continuation: NativeContinuation,
        _value: Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        // return one yielded continuation on the first resume
        if self.resume_calls == 0 {
            self.resume_calls += 1;
            return Ok(Outcome::Yielded {
                continuation: Continuation::Native(NativeContinuation::new(2)),
                value: Value::Void,
            });
        }

        // complete all later resumes
        self.resume_calls += 1;
        Ok(Outcome::Completed {
            output: void_output(),
        })
    }

    /// Test engines retain no heap roots.
    fn visit_roots(
        &mut self,
        _worker_static: &StaticSpace,
        _roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Test native continuations retain no heap roots.
    /// Capture one immutable engine image for tests.
    fn image(&mut self) -> RuntimeResult<NativeImage> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "test engine images are not implemented".to_string(),
        }
        .boxed())
    }

    /// Fork one live test engine.
    fn fork(&mut self, _heap: &mut heap::Heap) -> RuntimeResult<Box<dyn ErasedNativeEngine>> {
        Ok(Box::new(self.clone()))
    }

    /// Restore one immutable engine image for tests.
    fn restore(&mut self, _heap: &mut heap::Heap, image: &NativeImage) -> RuntimeResult<()> {
        let _ = image;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "test engine image restore is not implemented".to_string(),
        }
        .boxed())
    }
}

impl ErasedNativeEngine for AllocatingEngine {
    /// Run one entrypoint after allocating into the heap.
    fn run(
        &mut self,
        context: Context<'_>,
        _entry: &Entry,
        _args: &[Value],
    ) -> RuntimeResult<Outcome<Continuation>> {
        self.allocate(context.heap)?;

        Ok(Outcome::Completed {
            output: void_output(),
        })
    }

    /// Resume one continuation after allocating into the heap.
    fn resume(
        &mut self,
        context: Context<'_>,
        _continuation: NativeContinuation,
        _value: Value,
    ) -> RuntimeResult<Outcome<Continuation>> {
        self.allocate(context.heap)?;

        Ok(Outcome::Completed {
            output: void_output(),
        })
    }

    /// Allocating test engines retain no heap roots.
    fn visit_roots(
        &mut self,
        _worker_static: &StaticSpace,
        _roots: &mut RootVisitor<'_>,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Allocating test continuations retain no heap roots.
    /// Capture one immutable engine image for tests.
    fn image(&mut self) -> RuntimeResult<NativeImage> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "allocating test engine images are not implemented".to_string(),
        }
        .boxed())
    }

    /// Fork one live allocating test engine.
    fn fork(&mut self, _heap: &mut heap::Heap) -> RuntimeResult<Box<dyn ErasedNativeEngine>> {
        Ok(Box::new(*self))
    }

    /// Restore one immutable engine image for tests.
    fn restore(&mut self, _heap: &mut heap::Heap, image: &NativeImage) -> RuntimeResult<()> {
        let _ = image;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "allocating test engine image restore is not implemented".to_string(),
        }
        .boxed())
    }
}

impl AllocatingEngine {
    /// Allocate the configured heap and raw payload into the heap.
    fn allocate(&self, heap: &mut heap::Heap) -> RuntimeResult<()> {
        // heap payload
        if self.heap_values > 0 {
            let bytes = vec![0; self.heap_values * vm::Word::BYTE_LEN];
            let reference_map = ReferenceMap::empty();
            let layout = heap::AllocationLayout::new(bytes.len(), &reference_map);
            let _ = heap.allocate(layout, heap::Payload::Bytes(&bytes))?;
        }

        // raw payload
        if self.raw_bytes > 0 {
            let bytes = vec![0xAB; self.raw_bytes];
            let _ = heap.allocate_raw(bytes.len(), heap::Payload::Bytes(&bytes))?;
        }

        Ok(())
    }
}

/// Test harness for worker scheduling tests.
#[derive(Debug)]
pub(super) struct TestRuntime {
    /// Test world that owns the worker lifetime.
    world: World,
    /// Wrapped worker under test.
    worker: Worker,
    /// Runtime-owned shared heap state used by the worker.
    shared: RuntimeSharedHeap,
    /// Runtime-owned static bytes used by the worker.
    runtime_static: StaticSpace,
    /// Wrapped host under test.
    host: Session,
}

/// Test harness for multi-worker runtime scheduler tests.
#[derive(Debug)]
pub(super) struct TestMultiAgentRuntime {
    /// Test world that owns the runtime lifetime.
    world: World,
    /// Wrapped runtime identity under test.
    runtime_id: RuntimeId,
}

/// Test harness for world-level runtime and lineage tests.
#[derive(Debug)]
pub(super) struct TestWorld {
    /// Wrapped world under test.
    world: World,
}

/// Scripted poller for runtime ingress tests.
#[derive(Debug, Default)]
pub(super) struct TestPoller {
    /// Events returned by the next poll.
    events: Vec<PollerEvent>,
}

impl TestPoller {
    /// Build one scripted poller from explicit events.
    pub(super) fn with_events(events: Vec<PollerEvent>) -> Self {
        Self { events }
    }
}

impl TestWorld {
    /// Create one world with default runtime options.
    pub(super) fn new() -> Self {
        Self {
            world: World::from_options(&RuntimeOptions::default())
                .expect("runtime test world should build"),
        }
    }

    /// Create one world with explicit runtime options.
    pub(super) fn with_options(options: &RuntimeOptions) -> Self {
        let world = World::from_options(options).expect("runtime test world should build");

        Self { world }
    }

    /// Wrap one existing world.
    pub(super) fn from_world(world: World) -> Self {
        Self { world }
    }

    /// Borrow the wrapped world.
    pub(super) fn world(&self) -> &World {
        &self.world
    }

    /// Borrow the wrapped world mutably for test-only helpers.
    pub(super) fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Return the active branch metadata for the wrapped world.
    pub(super) fn branch(&self) -> Branch {
        self.world.branch()
    }

    /// Build one empty VM isolate for world tests.
    pub(super) fn vm_engine() -> vm::Isolate {
        let tree = NodeTree::new();
        let strings = LocalStringPool::new().into_immutable();

        vm::Isolate::build(vm::IsolateId::new(1), tree, strings).expect("vm engine should build")
    }

    /// Spawn one runtime with one explicit engine.
    pub(super) fn spawn_runtime(
        &mut self,
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> RuntimeId {
        self.world_mut()
            .spawn_runtime(Vec::new(), options, engine)
            .expect("runtime should spawn in world")
    }

    /// Spawn one VM-backed runtime.
    pub(super) fn spawn_vm_runtime(&mut self, options: &RuntimeOptions) -> RuntimeId {
        self.spawn_runtime(options, Self::vm_engine())
    }

    /// Return the primary worker id for one runtime.
    pub(super) fn primary_worker_id(&self, runtime_id: RuntimeId) -> WorkerId {
        self.world
            .runtime(runtime_id)
            .expect("runtime should exist")
            .primary_worker_id()
    }

    /// Allocate one heap value in the primary worker VM isolate.
    pub(super) fn allocate_vm_managed_value(
        &mut self,
        runtime_id: RuntimeId,
        value: vm::Word,
    ) -> heap::HeapReference {
        let runtime = self
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let engine = worker.engine.as_any_mut();
        let _isolate = engine
            .downcast_mut::<vm::Isolate>()
            .expect("worker should use a vm engine");
        let bytes = value.to_byte_array();
        let reference_map = ReferenceMap::empty();
        let layout = heap::AllocationLayout::new(bytes.len(), &reference_map);
        worker
            .heap
            .allocate(layout, heap::Payload::Bytes(&bytes))
            .expect("heap allocation should succeed")
    }

    /// Allocate one heap value in the primary worker VM isolate.
    pub(super) fn allocate_vm_heap_allocation(
        &mut self,
        runtime_id: RuntimeId,
    ) -> heap::HeapReference {
        self.allocate_vm_managed_value(runtime_id, vm::Word::int32(7))
    }

    /// Return the heap allocation count for the primary worker VM isolate.
    pub(super) fn vm_heap_allocation_count(&mut self, runtime_id: RuntimeId) -> usize {
        let runtime = self
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker.heap.heap_allocation_count()
    }

    /// Allocate one raw span in the primary worker heap.
    pub(super) fn allocate_vm_raw_bytes(
        &mut self,
        runtime_id: RuntimeId,
        bytes: &[u8],
    ) -> heap::RawPointer {
        let runtime = self
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker
            .heap
            .allocate_raw(bytes.len(), heap::Payload::Bytes(bytes))
            .expect("raw heap allocation should succeed")
    }

    /// Return the bytes for one raw allocation in the primary worker heap.
    pub(super) fn read_vm_raw_bytes_result(
        &mut self,
        runtime_id: RuntimeId,
        pointer: heap::RawPointer,
    ) -> heap::HeapResult<Vec<u8>> {
        let runtime = self
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker.heap.read_raw_bytes(pointer)
    }

    /// Return the bytes for one raw allocation in the primary worker heap.
    pub(super) fn read_vm_raw_bytes(
        &mut self,
        runtime_id: RuntimeId,
        pointer: heap::RawPointer,
    ) -> Vec<u8> {
        self.read_vm_raw_bytes_result(runtime_id, pointer)
            .expect("raw byte read should succeed")
    }

    /// Mutate one raw byte in the primary worker heap.
    pub(super) fn mutate_vm_raw_byte(
        &mut self,
        runtime_id: RuntimeId,
        pointer: heap::RawPointer,
        index: usize,
        byte: u8,
    ) {
        self.mutate_vm_raw_byte_result(runtime_id, pointer, index, byte)
            .expect("raw byte write should succeed");
    }

    /// Mutate one raw byte in the primary worker heap.
    pub(super) fn mutate_vm_raw_byte_result(
        &mut self,
        runtime_id: RuntimeId,
        pointer: heap::RawPointer,
        index: usize,
        byte: u8,
    ) -> heap::HeapResult<()> {
        let runtime = self
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker.heap.set_raw_byte(pointer, index, byte)
    }

    /// Capture the primary worker heap image for one runtime.
    pub(super) fn runtime_heap_image(&mut self, runtime_id: RuntimeId) -> heap::HeapImage {
        let runtime = self
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker
            .heap
            .image()
            .map_err(|error| {
                crate::diagnostic::RuntimeError::Internal {
                    message: format!("heap capture failed during world test: {error}"),
                }
                .boxed()
            })
            .expect("heap image capture should succeed")
    }

    /// Record one simple entity-kind topology mutation.
    pub(super) fn record_world_entity_kind(&mut self, suffix: &str) {
        self.world_mut()
            .define_entity_kind(WorldEntityKindDefinition {
                kind: format!("app.record.shared.{suffix}").into(),
                labels: Default::default(),
                supported_faults: Default::default(),
            })
            .expect("entity kind definition should succeed");
    }

    /// Commit one suspend revision for the wrapped world.
    pub(super) fn suspend(&mut self) -> Revision {
        self.world_mut()
            .suspend()
            .expect("world suspend should succeed")
    }

    /// Create one checkpoint on the wrapped world.
    pub(super) fn checkpoint(&mut self, name: &str) -> CheckpointId {
        self.world_mut()
            .checkpoint(name)
            .expect("world checkpoint should succeed")
    }

    /// Fork one child world from one checkpoint.
    pub(super) fn fork(&mut self, checkpoint_id: CheckpointId, name: &str) -> Self {
        let world = self
            .world_mut()
            .fork(checkpoint_id, name)
            .expect("world fork should succeed");

        Self::from_world(world)
    }
}

impl HostPoller for TestPoller {
    /// Registering resources is not used by these tests.
    fn register(
        &mut self,
        _resource_id: ResourceId,
        _handle: PlatformHandle,
        _token: PollerToken,
        _interests: PlatformInterest,
        _flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Updating resources is not used by these tests.
    fn update(
        &mut self,
        _resource_id: ResourceId,
        _token: PollerToken,
        _interests: PlatformInterest,
        _flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Deregistering resources is not used by these tests.
    fn deregister(&mut self, _resource_id: ResourceId) -> RuntimeResult<()> {
        Ok(())
    }

    /// Return no dedicated wake handle for this scripted poller.
    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        None
    }

    /// Waking the scripted poller is a no-op.
    fn wake(&mut self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Return the scripted poll result once and then drain it.
    fn poll(&mut self, _timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        Ok(std::mem::take(&mut self.events))
    }
}

impl TestRuntime {
    /// Create one test worker with default options.
    pub(super) fn new() -> Self {
        let (world, shared, runtime_static, worker, host) =
            agent_for_options_with_engine(&RuntimeOptions::default(), TestEngine::default());

        Self {
            world,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Create one test worker with explicit runtime options.
    #[allow(dead_code)]
    pub(super) fn with_options(options: &RuntimeOptions) -> Self {
        let (world, shared, runtime_static, worker, host) =
            agent_for_options_with_engine(options, TestEngine::default());

        Self {
            world,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Create one test worker with explicit options and one explicit engine.
    pub(super) fn with_options_and_engine(
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
    ) -> Self {
        let (world, shared, runtime_static, worker, host) =
            agent_for_options_with_engine(options, engine);

        Self {
            world,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Create one test worker with explicit options and one host clock source.
    #[allow(dead_code)]
    pub(super) fn with_options_and_host_clock_source(
        options: &RuntimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let (world, shared, runtime_static, worker, host) =
            agent_for_options_with_host_clock_source(options, Some(host_clock_source));

        Self {
            world,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Create one test worker with explicit options, one explicit engine, and one host clock source.
    pub(super) fn with_options_engine_and_host_clock_source(
        options: &RuntimeOptions,
        engine: impl Into<Engine>,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let (world, shared, runtime_static, worker, host) =
            agent_for_options_with_engine_and_host_clock_source(
                options,
                engine,
                Some(host_clock_source),
            );

        Self {
            world,
            shared,
            runtime_static,
            worker,
            host,
        }
    }

    /// Enqueue one native task with explicit identifiers.
    pub(super) fn enqueue_task_native(&mut self, task_id: u64, continuation_id: u64, priority: u8) {
        self.worker.event_loop.enqueue_task(Task {
            id: TaskId::new(task_id),
            runnable: Continuation::Native(NativeContinuation::new(continuation_handle(
                continuation_id,
            ))),
            resume_value: Value::Void,
            status: TaskStatus::Ready,
            priority,
        });
    }

    /// Enqueue one native microtask with explicit identifiers.
    pub(super) fn enqueue_microtask_native(&mut self, microtask_id: u64, continuation_id: u64) {
        self.worker.event_loop.enqueue_microtask(Microtask {
            id: MicrotaskId::new(microtask_id),
            continuation: Continuation::Native(NativeContinuation::new(continuation_handle(
                continuation_id,
            ))),
            resume_value: Value::Void,
            status: TaskStatus::Ready,
        });
    }

    /// Configure scheduler options and fail loudly in tests.
    pub(super) fn configure_scheduler(&mut self, options: SchedulerOptions) {
        self.worker
            .event_loop
            .configure(options)
            .expect("scheduler options should configure");
    }

    /// Borrow one execution view from the wrapped world.
    fn world_ref(&mut self) -> WorldRef {
        self.world.world_ref()
    }

    /// Return the exact live heap usage for this test worker.
    pub(super) fn heap_usage(&self) -> heap::HeapUsage {
        self.worker.heap.usage()
    }

    /// Replace the hard heap limits for this test worker.
    pub(super) fn set_heap_limits(&mut self, limits: heap::HeapLimits) {
        self.worker
            .heap
            .set_limits(limits)
            .expect("heap limits should configure");
    }

    /// Register one native timer watch.
    pub(super) fn watch_timer_native(&mut self, handle: u64, continuation_id: u64, priority: u8) {
        self.worker
            .watch_timer(
                ResourceId(handle),
                Continuation::Native(NativeContinuation::new(continuation_handle(
                    continuation_id,
                ))),
                Value::Void,
                priority,
            )
            .expect("timer watch should register");
    }

    /// Remove one timer watch and return whether one watch was present.
    pub(super) fn unwatch_timer(&mut self, handle: u64) -> bool {
        self.worker.unwatch_timer(ResourceId(handle)).is_some()
    }

    /// Schedule one timer in the event loop.
    pub(super) fn schedule_timer(
        &mut self,
        handle: u64,
        fire_at_nanos: u64,
        interval_nanos: Option<u64>,
    ) {
        self.schedule_timer_on(TimerClock::Wall, handle, fire_at_nanos, interval_nanos);
    }

    /// Schedule one timer in the event loop on one explicit clock domain.
    pub(super) fn schedule_timer_on(
        &mut self,
        clock: TimerClock,
        handle: u64,
        fire_at_nanos: u64,
        interval_nanos: Option<u64>,
    ) {
        self.worker
            .event_loop
            .schedule_timer(Timer {
                handle: ResourceId(handle).into(),
                deadline: TimerDeadline {
                    clock,
                    at: Nanos::new(fire_at_nanos),
                },
                interval: interval_nanos.map(Nanos::new),
            })
            .expect("timer should schedule");
    }

    /// Register one native event watch.
    pub(super) fn watch_event_native(&mut self, token: u64, continuation_id: u64, priority: u8) {
        self.worker
            .watch_event(
                PollerToken(token),
                Continuation::Native(NativeContinuation::new(continuation_handle(
                    continuation_id,
                ))),
                Value::Void,
                priority,
            )
            .expect("event watch should register");
    }

    /// Register one native host-event watch.
    pub(super) fn watch_host_event_native(
        &mut self,
        kind: HostEventKind,
        continuation_id: u64,
        priority: u8,
    ) {
        self.worker
            .watch_host_event(
                kind,
                Continuation::Native(NativeContinuation::new(continuation_handle(
                    continuation_id,
                ))),
                Value::Void,
                priority,
            )
            .expect("host event watch should register");
    }

    /// Enqueue one synthetic I/O event for dispatch tests.
    pub(super) fn enqueue_io_event(&mut self, resource_id: u64, token: u64, data: u64) {
        self.worker.event_loop.enqueue_events(vec![PollerEvent {
            resource_id: ResourceId(resource_id),
            source: PollerEventSource::Io,
            mask: PollerEventMask::READABLE,
            flags: PollerEventFlags::NONE,
            token: PollerToken(token),
            payload: PollerEventPayload::Io { data },
        }]);
    }

    /// Enqueue one synthetic lifecycle host event for dispatch tests.
    pub(super) fn enqueue_lifecycle_host_event(&mut self, state: HostLifecycleState) {
        self.worker
            .event_loop
            .enqueue_host_events(vec![HostEvent::Lifecycle(HostLifecycleEvent {
                source_kind: HostLifecycleSourceKind::Application,
                state,
            })]);
    }

    /// Tick once and fail loudly on runtime errors.
    pub(super) fn tick(&mut self) -> bool {
        let world = self.world_ref();

        self.worker
            .tick(&world, &self.shared, &self.runtime_static, &self.host)
            .expect("tick should execute runtime work")
    }

    /// Tick until idle and fail loudly on runtime errors.
    pub(super) fn tick_until_idle(&mut self) {
        let world = self.world_ref();

        let mut poller = None;
        self.worker
            .run_event_loop(
                &world,
                &self.shared,
                &self.runtime_static,
                &self.host,
                None,
                None,
                &mut poller,
            )
            .expect("tick until idle should complete");
    }

    /// Run one synthetic entrypoint and return the engine output.
    pub(super) fn run_entrypoint(&mut self) -> RuntimeResult<Output> {
        let world = self.world_ref();
        let mut poller = None;

        self.worker.run_entrypoint_with_host_and_poller(
            &world,
            &self.shared,
            &self.runtime_static,
            &self.host,
            &Entry::new("test.entry"),
            &[],
            &mut poller,
        )
    }

    /// Run until one task completes.
    pub(super) fn run_loop_until_task_complete(&mut self, task_id: u64) -> RuntimeResult<Output> {
        let world = self.world_ref();
        let mut poller = None;

        let output = self.worker.run_event_loop(
            &world,
            &self.shared,
            &self.runtime_static,
            &self.host,
            Some(TaskId::new(task_id)),
            None,
            &mut poller,
        )?;
        let Some(output) = output else {
            return Err(crate::diagnostic::RuntimeError::EventLoopIdle { task_id }.boxed());
        };

        Ok(output)
    }

    /// Run until one task completes or one timeout elapses.
    pub(super) fn run_loop_until_task_complete_with_timeout(
        &mut self,
        task_id: u64,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<Output>> {
        let world = self.world_ref();
        let mut poller = None;

        self.worker.run_event_loop(
            &world,
            &self.shared,
            &self.runtime_static,
            &self.host,
            Some(TaskId::new(task_id)),
            timeout_nanos,
            &mut poller,
        )
    }

    /// Return whether the event loop has pending work.
    pub(super) fn has_pending_work(&self) -> bool {
        self.worker.event_loop.has_pending_work()
    }

    /// Return whether the event loop has pending microtasks.
    pub(super) fn has_microtasks(&self) -> bool {
        self.worker.event_loop.has_microtasks()
    }

    /// Run one closure with one stored worker engine by explicit type.
    pub(super) fn with_engine<T: 'static, R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        let engine = self.worker.engine.as_any();
        let engine = engine
            .downcast_ref::<T>()
            .expect("worker engine should exist");

        callback(engine)
    }

    /// Return event-loop drop accounting.
    pub(super) fn drop_counts(&self) -> DropCounts {
        self.worker.drop_counts()
    }

    /// Return current runtime wall time in nanoseconds.
    pub(super) fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Return current runtime monotonic time in nanoseconds.
    pub(super) fn mono_nanos(&self) -> u64 {
        self.world.mono_nanos()
    }
}

impl TestMultiAgentRuntime {
    /// Create one runtime with explicit options and one explicit engine.
    pub(super) fn with_options_and_engine(
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

    /// Return the primary worker id.
    pub(super) fn primary_worker_id(&self) -> WorkerId {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .primary_worker_id()
    }

    /// Spawn one additional worker with one explicit engine and return its id.
    pub(super) fn spawn_worker(&mut self, engine: impl Into<Engine>) -> WorkerId {
        self.world
            .spawn_worker(self.runtime_id, engine)
            .expect("worker should spawn")
    }

    /// Run one closure with one mutable worker by id.
    pub(super) fn with_worker_mut<R>(
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
    pub(super) fn tick(&mut self) -> TickOutcome {
        self.world.tick().expect("runtime tick should succeed")
    }

    /// Attach one explicit scripted poller.
    pub(super) fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        let runtime = self
            .world
            .runtime_mut(self.runtime_id)
            .expect("runtime should exist");
        runtime.set_poller(poller);
    }

    /// Return runtime-level drop accounting.
    pub(super) fn drop_counts(&self) -> DropCounts {
        self.world
            .runtime(self.runtime_id)
            .expect("runtime should exist")
            .drop_counts()
    }

    /// Return current world wall time in nanoseconds.
    pub(super) fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Borrow the wrapped world.
    pub(crate) fn world(&self) -> &World {
        &self.world
    }

    /// Borrow the wrapped world mutably.
    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Run one closure with one stored primary-worker engine by explicit type.
    #[allow(dead_code)]
    pub(super) fn with_engine<T: 'static, R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        self.with_primary_engine(callback)
    }

    /// Return current world monotonic time in nanoseconds.
    pub(super) fn mono_nanos(&self) -> u64 {
        self.world.mono_nanos()
    }

    /// Run one closure with one stored primary-worker engine by explicit type.
    pub(super) fn with_primary_engine<T: 'static, R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let primary_worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker(primary_worker_id)
            .expect("primary worker should exist");
        let engine = worker.engine.as_any();
        let engine = engine
            .downcast_ref::<T>()
            .expect("worker engine should exist");

        callback(engine)
    }

    /// Run one closure with one stored worker engine by explicit type.
    pub(super) fn with_worker_engine<T: 'static, R>(
        &self,
        worker_id: WorkerId,
        callback: impl FnOnce(&T) -> R,
    ) -> R {
        let runtime = self
            .world
            .runtime(self.runtime_id)
            .expect("runtime should exist");
        let worker = runtime
            .worker(worker_id)
            .expect("worker should exist in runtime");
        let engine = worker.engine.as_any();
        let engine = engine
            .downcast_ref::<T>()
            .expect("worker engine should exist");

        callback(engine)
    }
}

/// Build one worker configured for runtime tests.
#[allow(dead_code)]
fn worker_for_options(
    options: &RuntimeOptions,
) -> (World, RuntimeSharedHeap, StaticSpace, Worker, Session) {
    agent_for_options_with_engine(options, TestEngine::default())
}

/// Build runtime-owned shared heap state for one test world.
pub(super) fn runtime_shared_heap(world: &World, options: &RuntimeOptions) -> RuntimeSharedHeap {
    let lineage = world.lineage.read();
    let shared = RuntimeSharedHeap::new(lineage.allocator(), lineage.collector(), options)
        .expect("runtime shared heap should build");

    shared
}

/// Build one worker configured for runtime tests and one optional host clock source.
#[allow(dead_code)]
fn agent_for_options_with_host_clock_source(
    options: &RuntimeOptions,
    host_clock_source: Option<Arc<dyn HostClockSource>>,
) -> (World, RuntimeSharedHeap, StaticSpace, Worker, Session) {
    agent_for_options_with_engine_and_host_clock_source(
        options,
        TestEngine::default(),
        host_clock_source,
    )
}

/// Build one worker configured for runtime tests with one explicit engine.
fn agent_for_options_with_engine(
    options: &RuntimeOptions,
    engine: impl Into<Engine>,
) -> (World, RuntimeSharedHeap, StaticSpace, Worker, Session) {
    agent_for_options_with_engine_and_host_clock_source(options, engine, None)
}

/// Build one worker configured for runtime tests with one explicit engine and one optional host clock source.
fn agent_for_options_with_engine_and_host_clock_source(
    options: &RuntimeOptions,
    engine: impl Into<Engine>,
    host_clock_source: Option<Arc<dyn HostClockSource>>,
) -> (World, RuntimeSharedHeap, StaticSpace, Worker, Session) {
    let mut world = if let Some(host_clock_source) = host_clock_source.clone() {
        World::new(options, Some(host_clock_source)).expect("runtime test world should build")
    } else {
        World::from_options(options).expect("runtime test world should build")
    };

    // construct one runtime worker from explicit options
    let world_ref = world.world_ref();
    let shared = runtime_shared_heap(&world, options);
    let runtime_static = StaticSpace::empty();
    let mut worker = Worker::new_in_world(
        Vec::new(),
        options,
        &world_ref,
        &shared,
        &runtime_static,
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

    // build the host for this test worker
    let host = Session::from_runtime_options(options, worker.runtime_id);

    // drain initial host bootstrap events for deterministic scheduler tests
    host.poll(Some(0))
        .expect("host bootstrap events should drain");

    (world, shared, runtime_static, worker, host)
}

/// Build one void runtime output.
fn void_output() -> Output {
    Output { value: Value::Void }
}

/// Convert one test continuation identifier into one native continuation handle.
fn continuation_handle(value: u64) -> usize {
    usize::try_from(value).expect("test continuation id should fit usize")
}
