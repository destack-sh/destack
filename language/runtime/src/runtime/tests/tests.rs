use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_core::LocalStringPool;
use destack_mir::NodeTree;
use destack_workspace::{RuntimeOptions, SchedulerOptions};
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::RuntimeResult;
use crate::host::{
    HostEvent, HostEventKind, HostLifecycleEvent, HostLifecycleSourceKind, HostLifecycleState,
    HostSession,
};
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineContinuationImage, EngineImage, EngineSnapshot, Entry, Entry,
    ExecutionOutcome, ExecutionOutput, NativeContinuation,
};
use crate::runtime::poller::{
    HostPoller, HostPollerFlags, PlatformHandle, PlatformInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::runtime::scheduler::{
    Microtask, MicrotaskId, Task, TaskId, TaskStatus, Timer, TimerDeadline,
};
use crate::runtime::time::{HostClockSource, Nanos};
use crate::runtime::world::{Branch, CheckpointId, RevisionId, WorldEntityKindDefinition};
use crate::runtime::{Agent, AgentId, DropCounts, RuntimeId, TickOutcome, World};

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
#[derive(Debug, Default)]
pub(super) struct TestEngine {
    /// Number of resume calls executed.
    pub(super) resume_calls: usize,
}

/// Engine that allocates into the heap when it runs or resumes.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AllocatingEngine {
    /// The number of managed values to allocate into one managed allocation.
    pub(super) managed_values: usize,
    /// The number of raw bytes to allocate into one raw allocation.
    pub(super) raw_bytes: usize,
}

impl Engine for TestEngine {
    /// Run one entrypoint without yielding.
    fn run(
        &mut self,
        _memory: &mut heap::MemoryContext<'_>,
        _entry: &Entry,
        _args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        Ok(ExecutionOutcome::Completed {
            output: void_output(),
        })
    }

    /// Run one replayable entrypoint without yielding.
    fn run_replayable_entry(
        &mut self,
        _memory: &mut heap::MemoryContext<'_>,
        _entry: &Entry,
        _args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        Ok(ExecutionOutcome::Completed {
            output: void_output(),
        })
    }

    /// Resume one continuation and yield once before completion.
    fn resume(
        &mut self,
        _memory: &mut heap::MemoryContext<'_>,
        _continuation: EngineContinuation,
        _value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        // return one yielded continuation on the first resume
        if self.resume_calls == 0 {
            self.resume_calls += 1;
            return Ok(ExecutionOutcome::Yielded {
                yielded: destack_engine::ExecutionYield {
                    continuation: EngineContinuation::Native(NativeContinuation::new(2)),
                    value: heap::Value::VOID,
                },
            });
        }

        // complete all later resumes
        self.resume_calls += 1;
        Ok(ExecutionOutcome::Completed {
            output: void_output(),
        })
    }

    /// Capture one immutable engine image for tests.
    fn image(&mut self) -> RuntimeResult<EngineImage> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "test engine images are not implemented".to_string(),
        }
        .boxed())
    }

    /// Restore one immutable engine image for tests.
    fn restore_image(&mut self, _heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()> {
        let _ = image;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "test engine image restore is not implemented".to_string(),
        }
        .boxed())
    }

    /// Capture one continuation image for tests.
    fn continuation_image(
        &mut self,
        continuation: &EngineContinuation,
    ) -> RuntimeResult<EngineContinuationImage> {
        match continuation {
            EngineContinuation::Native(continuation) => {
                Ok(EngineContinuationImage::Native(*continuation))
            }
            EngineContinuation::Vm(_) => Err(crate::diagnostic::RuntimeError::Internal {
                message: "test engine vm continuation images are not implemented".to_string(),
            }
            .boxed()),
        }
    }

    /// Restore one continuation image for tests.
    fn restore_continuation_image(
        &mut self,
        image: &EngineContinuationImage,
    ) -> RuntimeResult<EngineContinuation> {
        match image {
            EngineContinuationImage::Native(continuation) => {
                Ok(EngineContinuation::Native(*continuation))
            }
            EngineContinuationImage::Vm(_) => Err(crate::diagnostic::RuntimeError::Internal {
                message: "test engine vm continuation restore is not implemented".to_string(),
            }
            .boxed()),
        }
    }

    /// Capture one serialized engine snapshot for tests.
    fn snapshot(&mut self) -> RuntimeResult<EngineSnapshot> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "test engine snapshots are not implemented".to_string(),
        }
        .boxed())
    }

    /// Restore one serialized engine snapshot for tests.
    fn restore_snapshot(
        &mut self,
        _heap: &mut heap::Heap,
        snapshot: &EngineSnapshot,
    ) -> RuntimeResult<()> {
        let _ = snapshot;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "test engine snapshot restore is not implemented".to_string(),
        }
        .boxed())
    }
}

impl Engine for AllocatingEngine {
    /// Run one entrypoint after allocating into the heap.
    fn run(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        _entry: &Entry,
        _args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        self.allocate(memory.heap())?;

        Ok(ExecutionOutcome::Completed {
            output: void_output(),
        })
    }

    /// Run one replayable entrypoint after allocating into the heap.
    fn run_replayable_entry(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        _entry: &Entry,
        _args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        self.allocate(memory.heap())?;

        Ok(ExecutionOutcome::Completed {
            output: void_output(),
        })
    }

    /// Resume one continuation after allocating into the heap.
    fn resume(
        &mut self,
        memory: &mut heap::MemoryContext<'_>,
        _continuation: EngineContinuation,
        _value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<EngineContinuation>> {
        self.allocate(memory.heap())?;

        Ok(ExecutionOutcome::Completed {
            output: void_output(),
        })
    }

    /// Capture one immutable engine image for tests.
    fn image(&mut self) -> RuntimeResult<EngineImage> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "allocating test engine images are not implemented".to_string(),
        }
        .boxed())
    }

    /// Restore one immutable engine image for tests.
    fn restore_image(&mut self, _heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()> {
        let _ = image;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "allocating test engine image restore is not implemented".to_string(),
        }
        .boxed())
    }

    /// Capture one continuation image for tests.
    fn continuation_image(
        &mut self,
        continuation: &EngineContinuation,
    ) -> RuntimeResult<EngineContinuationImage> {
        match continuation {
            EngineContinuation::Native(continuation) => {
                Ok(EngineContinuationImage::Native(*continuation))
            }
            EngineContinuation::Vm(_) => Err(crate::diagnostic::RuntimeError::Internal {
                message: "allocating test engine vm continuation images are not implemented"
                    .to_string(),
            }
            .boxed()),
        }
    }

    /// Restore one continuation image for tests.
    fn restore_continuation_image(
        &mut self,
        image: &EngineContinuationImage,
    ) -> RuntimeResult<EngineContinuation> {
        match image {
            EngineContinuationImage::Native(continuation) => {
                Ok(EngineContinuation::Native(*continuation))
            }
            EngineContinuationImage::Vm(_) => Err(crate::diagnostic::RuntimeError::Internal {
                message: "allocating test engine vm continuation restore is not implemented"
                    .to_string(),
            }
            .boxed()),
        }
    }

    /// Capture one serialized engine snapshot for tests.
    fn snapshot(&mut self) -> RuntimeResult<EngineSnapshot> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "allocating test engine snapshots are not implemented".to_string(),
        }
        .boxed())
    }

    /// Restore one serialized engine snapshot for tests.
    fn restore_snapshot(
        &mut self,
        _heap: &mut heap::Heap,
        snapshot: &EngineSnapshot,
    ) -> RuntimeResult<()> {
        let _ = snapshot;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "allocating test engine snapshot restore is not implemented".to_string(),
        }
        .boxed())
    }
}

impl AllocatingEngine {
    /// Allocate the configured managed and raw payload into the heap.
    fn allocate(&self, heap: &mut heap::Heap) -> RuntimeResult<()> {
        // managed payload
        if self.managed_values > 0 {
            let mut values = Vec::with_capacity(self.managed_values);
            values.resize(self.managed_values, heap::Value::int64(7));
            let _ = heap.allocate_packed_values(values)?;
        }

        // raw payload
        if self.raw_bytes > 0 {
            let bytes = vec![0xAB; self.raw_bytes];
            let _ = heap.allocate_raw_bytes(&bytes)?;
        }

        Ok(())
    }
}

/// Test harness for agent scheduling tests.
#[derive(Debug)]
pub(super) struct TestRuntime {
    /// Shared world that owns the agent lifetime.
    world: Arc<World>,
    /// Wrapped agent under test.
    agent: Agent,
    /// Wrapped host under test.
    host: HostSession,
}

/// Test harness for multi-agent runtime scheduler tests.
#[derive(Debug)]
pub(super) struct TestMultiAgentRuntime {
    /// Shared world that owns the runtime lifetime.
    world: Arc<World>,
    /// Wrapped runtime identity under test.
    runtime_id: RuntimeId,
}

/// Test harness for world-level runtime and lineage tests.
#[derive(Debug, Clone)]
pub(super) struct TestWorld {
    /// Wrapped world under test.
    world: Arc<World>,
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
    #[allow(clippy::arc_with_non_send_sync)]
    /// Create one world with default runtime options.
    pub(super) fn new() -> Self {
        Self {
            world: Arc::new(World::default()),
        }
    }

    /// Create one world with explicit runtime options.
    pub(super) fn with_options(options: &RuntimeOptions) -> Self {
        let world = World::from_options(options).expect("runtime test world should build");

        Self { world }
    }

    /// Wrap one existing shared world.
    pub(super) fn from_world(world: Arc<World>) -> Self {
        Self { world }
    }

    /// Borrow the wrapped world.
    pub(super) fn world(&self) -> Arc<World> {
        Arc::clone(&self.world)
    }

    /// Return the active branch metadata for the wrapped world.
    pub(super) fn branch(&self) -> Branch {
        self.world.branch()
    }

    /// Build one empty VM isolate for world tests.
    pub(super) fn vm_engine() -> vm::Isolate {
        let tree = NodeTree::new();
        let strings = LocalStringPool::new().into_immutable();

        vm::Isolate::build(tree, strings).expect("vm engine should build")
    }

    /// Spawn one runtime with one explicit engine.
    pub(super) fn spawn_runtime(
        &self,
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> RuntimeId {
        self.world
            .spawn_runtime(Vec::new(), options, engine)
            .expect("runtime should spawn in world")
    }

    /// Spawn one VM-backed runtime.
    pub(super) fn spawn_vm_runtime(&self, options: &RuntimeOptions) -> RuntimeId {
        self.spawn_runtime(options, Self::vm_engine())
    }

    /// Return the primary agent id for one runtime.
    pub(super) fn primary_agent_id(&self, runtime_id: RuntimeId) -> AgentId {
        self.world
            .with_runtime(runtime_id, |runtime| Ok(runtime.primary_agent_id()))
            .expect("runtime should exist")
    }

    /// Allocate one managed heap value in the primary agent VM isolate.
    pub(super) fn allocate_vm_managed_value(&self, runtime_id: RuntimeId, value: heap::Value) {
        self.world
            .with_runtime_mut(runtime_id, |runtime| {
                let agent_id = runtime.primary_agent_id();
                let agent = runtime
                    .agent_mut(agent_id)
                    .expect("runtime should keep its primary agent");
                let engine = &mut *agent.engine as &mut dyn std::any::Any;
                let isolate = engine
                    .downcast_mut::<vm::Isolate>()
                    .expect("agent should use a vm engine");
                let _ = isolate.allocate_single(&mut agent.heap, value);

                Ok(())
            })
            .expect("vm heap mutation should succeed");
    }

    /// Allocate one managed heap value in the primary agent VM isolate.
    pub(super) fn allocate_vm_heap_allocation(&self, runtime_id: RuntimeId) {
        self.allocate_vm_managed_value(runtime_id, heap::Value::int32(7));
    }

    /// Return the managed heap allocation count for the primary agent VM isolate.
    pub(super) fn vm_heap_allocation_count(&self, runtime_id: RuntimeId) -> usize {
        self.world
            .with_runtime_mut(runtime_id, |runtime| {
                let agent_id = runtime.primary_agent_id();
                let agent = runtime
                    .agent_mut(agent_id)
                    .expect("runtime should keep its primary agent");

                Ok(agent.heap.managed_allocation_count())
            })
            .expect("vm heap inspection should succeed")
    }

    /// Allocate one raw span in the primary agent heap.
    pub(super) fn allocate_vm_raw_bytes(
        &self,
        runtime_id: RuntimeId,
        bytes: &[u8],
    ) -> heap::RawPointer {
        self.world
            .with_runtime_mut(runtime_id, |runtime| {
                let agent_id = runtime.primary_agent_id();
                let agent = runtime
                    .agent_mut(agent_id)
                    .expect("runtime should keep its primary agent");

                let pointer = agent
                    .heap
                    .allocate_raw_bytes(bytes)
                    .expect("raw heap allocation should succeed");

                Ok(pointer)
            })
            .expect("raw heap allocation should succeed")
    }

    /// Mutate one raw byte in the primary agent heap.
    pub(super) fn mutate_vm_raw_byte(
        &self,
        runtime_id: RuntimeId,
        pointer: heap::RawPointer,
        index: usize,
        byte: u8,
    ) {
        self.world
            .with_runtime_mut(runtime_id, |runtime| {
                let agent_id = runtime.primary_agent_id();
                let agent = runtime
                    .agent_mut(agent_id)
                    .expect("runtime should keep its primary agent");

                assert!(agent.heap.set_raw_byte(pointer, index, byte));
                Ok(())
            })
            .expect("raw heap mutation should succeed");
    }

    /// Capture the primary agent heap image for one runtime.
    pub(super) fn runtime_heap_image(&self, runtime_id: RuntimeId) -> heap::HeapImage {
        self.world
            .with_runtime_mut(runtime_id, |runtime| {
                let agent_id = runtime.primary_agent_id();
                let agent = runtime
                    .agent_mut(agent_id)
                    .expect("runtime should keep its primary agent");

                agent.heap.image().map_err(|error| {
                    crate::diagnostic::RuntimeError::Internal {
                        message: format!("heap capture failed during world test: {error}"),
                    }
                    .boxed()
                })
            })
            .expect("heap image capture should succeed")
    }

    /// Record one simple entity-kind topology mutation.
    pub(super) fn record_world_entity_kind(&self, suffix: &str) {
        self.world
            .define_entity_kind(WorldEntityKindDefinition {
                kind: format!("app.record.shared.{suffix}").into(),
                labels: Default::default(),
                supported_faults: Default::default(),
            })
            .expect("entity kind definition should succeed");
    }

    /// Commit one suspend revision for the wrapped world.
    pub(super) fn suspend(&self) -> RevisionId {
        self.world.suspend().expect("world suspend should succeed")
    }

    /// Create one checkpoint on the wrapped world.
    pub(super) fn checkpoint(&self, name: &str) -> CheckpointId {
        self.world
            .checkpoint(name)
            .expect("world checkpoint should succeed")
    }

    /// Fork one child world from one checkpoint.
    pub(super) fn fork(&self, checkpoint_id: CheckpointId, name: &str) -> Self {
        let world = self
            .world
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
    /// Create one test agent with default options.
    pub(super) fn new() -> Self {
        let (world, agent, host) =
            agent_for_options_with_engine(&RuntimeOptions::default(), TestEngine::default());

        Self { world, agent, host }
    }

    /// Create one test agent with explicit runtime options.
    #[allow(dead_code)]
    pub(super) fn with_options(options: &RuntimeOptions) -> Self {
        let (world, agent, host) = agent_for_options_with_engine(options, TestEngine::default());

        Self { world, agent, host }
    }

    /// Create one test agent with explicit options and one explicit engine.
    pub(super) fn with_options_and_engine(
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> Self {
        let (world, agent, host) = agent_for_options_with_engine(options, engine);

        Self { world, agent, host }
    }

    /// Create one test agent with explicit options and one host clock source.
    #[allow(dead_code)]
    pub(super) fn with_options_and_host_clock_source(
        options: &RuntimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let (world, agent, host) =
            agent_for_options_with_host_clock_source(options, Some(host_clock_source));

        Self { world, agent, host }
    }

    /// Create one test agent with explicit options, one explicit engine, and one host clock source.
    pub(super) fn with_options_engine_and_host_clock_source(
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let (world, agent, host) = agent_for_options_with_engine_and_host_clock_source(
            options,
            engine,
            Some(host_clock_source),
        );

        Self { world, agent, host }
    }

    /// Enqueue one native task with explicit identifiers.
    pub(super) fn enqueue_task_native(&mut self, task_id: u64, continuation_id: u64, priority: u8) {
        self.agent.event_loop.enqueue_task(Task {
            id: TaskId::new(task_id),
            runnable: EngineContinuation::Native(NativeContinuation::new(continuation_handle(
                continuation_id,
            ))),
            resume_value: heap::Value::VOID,
            status: TaskStatus::Ready,
            priority,
        });
    }

    /// Enqueue one native microtask with explicit identifiers.
    pub(super) fn enqueue_microtask_native(&mut self, microtask_id: u64, continuation_id: u64) {
        self.agent.event_loop.enqueue_microtask(Microtask {
            id: MicrotaskId::new(microtask_id),
            continuation: EngineContinuation::Native(NativeContinuation::new(continuation_handle(
                continuation_id,
            ))),
            resume_value: heap::Value::VOID,
            status: TaskStatus::Ready,
        });
    }

    /// Configure scheduler options and fail loudly in tests.
    pub(super) fn configure_scheduler(&mut self, options: SchedulerOptions) {
        self.agent
            .event_loop
            .configure(options)
            .expect("scheduler options should configure");
    }

    /// Return the exact live heap usage for this test agent.
    pub(super) fn heap_usage(&self) -> heap::HeapUsage {
        self.agent.heap.usage()
    }

    /// Replace the hard heap limits for this test agent.
    pub(super) fn set_heap_limits(&mut self, limits: heap::HeapLimits) {
        self.agent
            .heap
            .set_limits(limits)
            .expect("heap limits should configure");
    }

    /// Register one native timer watch.
    pub(super) fn watch_timer_native(&mut self, handle: u64, continuation_id: u64, priority: u8) {
        self.agent
            .watch_timer(
                ResourceId(handle),
                EngineContinuation::Native(NativeContinuation::new(continuation_handle(
                    continuation_id,
                ))),
                heap::Value::VOID,
                priority,
            )
            .expect("timer watch should register");
    }

    /// Remove one timer watch and return whether one watch was present.
    pub(super) fn unwatch_timer(&mut self, handle: u64) -> bool {
        self.agent.unwatch_timer(ResourceId(handle)).is_some()
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
        self.agent
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
        self.agent
            .watch_event(
                PollerToken(token),
                EngineContinuation::Native(NativeContinuation::new(continuation_handle(
                    continuation_id,
                ))),
                heap::Value::VOID,
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
        self.agent
            .watch_host_event(
                kind,
                EngineContinuation::Native(NativeContinuation::new(continuation_handle(
                    continuation_id,
                ))),
                heap::Value::VOID,
                priority,
            )
            .expect("host event watch should register");
    }

    /// Enqueue one synthetic I/O event for dispatch tests.
    pub(super) fn enqueue_io_event(&mut self, resource_id: u64, token: u64, data: u64) {
        self.agent.event_loop.enqueue_events(vec![PollerEvent {
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
        self.agent
            .event_loop
            .enqueue_host_events(vec![HostEvent::Lifecycle(HostLifecycleEvent {
                source_kind: HostLifecycleSourceKind::Application,
                state,
            })]);
    }

    /// Tick once and fail loudly on runtime errors.
    pub(super) fn tick(&mut self) -> bool {
        self.agent
            .tick(&self.world, &self.host)
            .expect("tick should execute runtime work")
    }

    /// Tick until idle and fail loudly on runtime errors.
    pub(super) fn tick_until_idle(&mut self) {
        self.agent
            .tick_until_idle(&self.world, &self.host)
            .expect("tick until idle should complete");
    }

    /// Run one synthetic entrypoint and return the engine output.
    pub(super) fn run_entrypoint(&mut self) -> RuntimeResult<ExecutionOutput> {
        self.agent
            .run_entrypoint(&self.world, &self.host, &Entry::new("test.entry"), &[])
    }

    /// Run until one task completes.
    pub(super) fn run_loop_until_task_complete(
        &mut self,
        task_id: u64,
    ) -> RuntimeResult<ExecutionOutput> {
        self.agent
            .run_loop_until_task_complete(&self.world, &self.host, TaskId::new(task_id))
    }

    /// Run until one task completes or one timeout elapses.
    pub(super) fn run_loop_until_task_complete_with_timeout(
        &mut self,
        task_id: u64,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<ExecutionOutput>> {
        self.agent.run_loop_until_task_complete_with_timeout(
            &self.world,
            &self.host,
            TaskId::new(task_id),
            timeout_nanos,
        )
    }

    /// Return whether the event loop has pending work.
    pub(super) fn has_pending_work(&self) -> bool {
        self.agent.event_loop.has_pending_work()
    }

    /// Return whether the event loop has pending microtasks.
    pub(super) fn has_microtasks(&self) -> bool {
        self.agent.event_loop.has_microtasks()
    }

    /// Run one closure with one stored agent engine by explicit type.
    pub(super) fn with_engine<T: Engine, R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        let engine = self.agent.engine.as_ref() as &dyn std::any::Any;
        let engine = engine
            .downcast_ref::<T>()
            .expect("agent engine should exist");

        callback(engine)
    }

    /// Return event-loop drop accounting.
    pub(super) fn drop_counts(&self) -> DropCounts {
        self.agent.drop_counts()
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
        engine: impl Engine + 'static,
    ) -> Self {
        let world = World::from_options(options).expect("world should build");
        let runtime_id = world
            .spawn_runtime(Vec::new(), options, engine)
            .expect("runtime should spawn");
        world
            .with_runtime(runtime_id, |runtime| {
                runtime
                    .host()
                    .poll_events(Some(0))
                    .expect("host bootstrap events should drain");
                Ok(())
            })
            .expect("runtime should exist");

        Self { world, runtime_id }
    }

    /// Return the primary agent id.
    pub(super) fn primary_agent_id(&self) -> AgentId {
        self.world
            .with_runtime(self.runtime_id, |runtime| Ok(runtime.primary_agent_id()))
            .expect("runtime should exist")
    }

    /// Spawn one additional agent with one explicit engine and return its id.
    pub(super) fn spawn_agent(&mut self, engine: impl Engine + 'static) -> AgentId {
        self.world
            .spawn_agent(self.runtime_id, engine)
            .expect("agent should spawn")
    }

    /// Run one closure with one mutable agent by id.
    pub(super) fn with_agent_mut<R>(
        &mut self,
        agent_id: AgentId,
        callback: impl FnOnce(&mut Agent) -> R,
    ) -> R {
        self.world
            .with_runtime_mut(self.runtime_id, |runtime| {
                let agent = runtime
                    .agent_mut(agent_id)
                    .expect("agent should exist in runtime");

                Ok(callback(agent))
            })
            .expect("runtime should exist")
    }

    /// Execute one runtime tick and fail loudly on runtime errors.
    pub(super) fn tick(&mut self) -> TickOutcome {
        self.world
            .tick_runtime(self.runtime_id)
            .expect("runtime tick should succeed")
    }

    /// Attach one explicit scripted poller.
    pub(super) fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        self.world
            .with_runtime_mut(self.runtime_id, |runtime| {
                runtime.set_poller(poller);
                Ok(())
            })
            .expect("runtime should exist");
    }

    /// Return runtime-level drop accounting.
    pub(super) fn drop_counts(&self) -> DropCounts {
        self.world
            .with_runtime(self.runtime_id, |runtime| Ok(runtime.drop_counts()))
            .expect("runtime should exist")
    }

    /// Return current world wall time in nanoseconds.
    pub(super) fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Borrow the shared world.
    pub(crate) fn world(&self) -> Arc<World> {
        self.world.clone()
    }

    /// Run one closure with one stored primary-agent engine by explicit type.
    #[allow(dead_code)]
    pub(super) fn with_engine<T: Engine, R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        self.with_primary_engine(callback)
    }

    /// Return current world monotonic time in nanoseconds.
    pub(super) fn mono_nanos(&self) -> u64 {
        self.world.mono_nanos()
    }

    /// Run one closure with one stored primary-agent engine by explicit type.
    pub(super) fn with_primary_engine<T: Engine, R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        self.world
            .with_runtime(self.runtime_id, |runtime| {
                let primary_agent_id = runtime.primary_agent_id();
                let agent = runtime
                    .agent(primary_agent_id)
                    .expect("primary agent should exist");
                let engine = agent.engine.as_ref() as &dyn std::any::Any;
                let engine = engine
                    .downcast_ref::<T>()
                    .expect("agent engine should exist");
                Ok(callback(engine))
            })
            .expect("runtime should exist")
    }

    /// Run one closure with one stored agent engine by explicit type.
    pub(super) fn with_agent_engine<T: Engine, R>(
        &self,
        agent_id: AgentId,
        callback: impl FnOnce(&T) -> R,
    ) -> R {
        self.world
            .with_runtime(self.runtime_id, |runtime| {
                let agent = runtime
                    .agent(agent_id)
                    .expect("agent should exist in runtime");
                let engine = agent.engine.as_ref() as &dyn std::any::Any;
                let engine = engine
                    .downcast_ref::<T>()
                    .expect("agent engine should exist");
                Ok(callback(engine))
            })
            .expect("runtime should exist")
    }
}

/// Build one agent configured for runtime tests.
#[allow(dead_code)]
fn agent_for_options(options: &RuntimeOptions) -> (Arc<World>, Agent, HostSession) {
    agent_for_options_with_engine(options, TestEngine::default())
}

/// Build one agent configured for runtime tests and one optional host clock source.
#[allow(dead_code)]
fn agent_for_options_with_host_clock_source(
    options: &RuntimeOptions,
    host_clock_source: Option<Arc<dyn HostClockSource>>,
) -> (Arc<World>, Agent, HostSession) {
    agent_for_options_with_engine_and_host_clock_source(
        options,
        TestEngine::default(),
        host_clock_source,
    )
}

/// Build one agent configured for runtime tests with one explicit engine.
fn agent_for_options_with_engine(
    options: &RuntimeOptions,
    engine: impl Engine + 'static,
) -> (Arc<World>, Agent, HostSession) {
    agent_for_options_with_engine_and_host_clock_source(options, engine, None)
}

/// Build one agent configured for runtime tests with one explicit engine and one optional host clock source.
fn agent_for_options_with_engine_and_host_clock_source(
    options: &RuntimeOptions,
    engine: impl Engine + 'static,
    host_clock_source: Option<Arc<dyn HostClockSource>>,
) -> (Arc<World>, Agent, HostSession) {
    let world = if let Some(host_clock_source) = host_clock_source.clone() {
        World::new(options, Some(host_clock_source)).expect("runtime test world should build")
    } else {
        World::from_options(options).expect("runtime test world should build")
    };

    // construct one runtime agent from explicit options
    let mut agent = Agent::new_in_world(Vec::new(), options, &world, Box::new(engine))
        .expect("runtime test agent should build");

    // configure scheduler options for deterministic tests
    agent
        .event_loop
        .configure(options.scheduler.clone())
        .expect("scheduler options should configure");

    // apply runtime options to binding policy state
    agent.bindings.apply_runtime_defaults(options);

    // build the host for this test agent
    let host = HostSession::from_runtime_options(options, agent.runtime_id);

    // drain initial host bootstrap events for deterministic scheduler tests
    host.poll_events(Some(0))
        .expect("host bootstrap events should drain");

    (world, agent, host)
}

/// Build one void runtime output.
fn void_output() -> ExecutionOutput {
    ExecutionOutput {
        value: heap::Value::VOID,
        stats: Default::default(),
        managed_allocation_count: 0,
        raw_allocation_count: 0,
    }
}

/// Convert one test continuation identifier into one native continuation handle.
fn continuation_handle(value: u64) -> usize {
    usize::try_from(value).expect("test continuation id should fit usize")
}
