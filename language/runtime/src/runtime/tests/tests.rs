use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_heap as heap;
use destack_workspace::{RuntimeOptions, SchedulerOptions};

use crate::diagnostic::RuntimeResult;
use crate::host::{Host, HostEvent, HostEventKind, HostLifecycleEvent, HostLifecycleState};
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::Agent;
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, EngineOutput, NativeContinuation,
};
use crate::runtime::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::runtime::scheduler::{Microtask, MicrotaskId, Task, TaskId, TaskStatus, Timer};
use crate::runtime::time::HostClockSource;

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

impl Engine for TestEngine {
    /// Entry payload for this engine.
    type Entry = ();

    /// Run one entrypoint without yielding.
    fn run(&mut self, _entry: &Self::Entry, _args: &[heap::Value]) -> RuntimeResult<EngineOutcome> {
        Ok(EngineOutcome::Completed {
            output: void_output(),
        })
    }

    /// Resume one continuation and yield once before completion.
    fn resume(
        &mut self,
        _continuation: EngineContinuation,
        _value: heap::Value,
    ) -> RuntimeResult<EngineOutcome> {
        // return one yielded continuation on the first resume
        if self.resume_calls == 0 {
            self.resume_calls += 1;
            return Ok(EngineOutcome::Yielded {
                continuation: EngineContinuation::Native(NativeContinuation::new(2)),
                value: heap::Value::VOID,
            });
        }

        // complete all later resumes
        self.resume_calls += 1;
        Ok(EngineOutcome::Completed {
            output: void_output(),
        })
    }
}

/// Test harness for agent scheduling tests.
#[derive(Debug)]
pub(super) struct TestRuntime {
    /// Wrapped agent under test.
    agent: Agent,
    /// Wrapped host under test.
    host: Host,
}

impl TestRuntime {
    /// Create one test agent with default options.
    pub(super) fn new() -> Self {
        let (agent, host) = agent_for_options(&RuntimeOptions::default());

        Self { agent, host }
    }

    /// Create one test agent with explicit runtime options.
    pub(super) fn with_options(options: &RuntimeOptions) -> Self {
        let (agent, host) = agent_for_options(options);

        Self { agent, host }
    }

    /// Create one test agent with explicit options and one host clock source.
    pub(super) fn with_options_and_host_clock_source(
        options: &RuntimeOptions,
        host_clock_source: Arc<dyn HostClockSource>,
    ) -> Self {
        let (agent, host) =
            agent_for_options_with_host_clock_source(options, Some(host_clock_source));

        Self { agent, host }
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
                clock,
                handle: ResourceId(handle),
                fire_at_nanos,
                interval_nanos,
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
            .enqueue_host_events(vec![HostEvent::Lifecycle(HostLifecycleEvent { state })]);
    }

    /// Tick once and fail loudly on runtime errors.
    pub(super) fn tick_once<E: Engine>(&mut self, engine: &mut E) -> bool {
        self.agent
            .tick_once(&self.host, engine)
            .expect("tick should execute runtime work")
    }

    /// Tick until idle and fail loudly on runtime errors.
    pub(super) fn tick_until_idle<E: Engine>(&mut self, engine: &mut E) {
        self.agent
            .tick_until_idle(&self.host, engine)
            .expect("tick until idle should complete");
    }

    /// Run until one task completes.
    pub(super) fn run_loop_until_task_complete<E: Engine>(
        &mut self,
        engine: &mut E,
        task_id: u64,
    ) -> RuntimeResult<EngineOutput> {
        self.agent
            .run_loop_until_task_complete(&self.host, engine, TaskId::new(task_id))
    }

    /// Run until one task completes or one timeout elapses.
    pub(super) fn run_loop_until_task_complete_with_timeout<E: Engine>(
        &mut self,
        engine: &mut E,
        task_id: u64,
        timeout_nanos: Option<u64>,
    ) -> RuntimeResult<Option<EngineOutput>> {
        self.agent.run_loop_until_task_complete_with_timeout(
            &self.host,
            engine,
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

    /// Return dropped dispatch-event count.
    pub(super) fn dropped_dispatch_events(&self) -> u64 {
        self.agent.event_loop.dropped_dispatch_events()
    }

    /// Return dropped dispatch-event count for events without a watch.
    pub(super) fn dropped_unwatched_dispatch_events(&self) -> u64 {
        self.agent.event_loop.dropped_unwatched_dispatch_events()
    }

    /// Return dropped dispatch-event count from host queue pressure.
    pub(super) fn dropped_host_queue_events(&self) -> u64 {
        self.agent.event_loop.dropped_host_queue_events()
    }

    /// Return current runtime wall time in nanoseconds.
    pub(super) fn wall_nanos(&self) -> u64 {
        self.agent.world().wall_nanos()
    }

    /// Return current runtime monotonic time in nanoseconds.
    pub(super) fn mono_nanos(&self) -> u64 {
        self.agent.world().mono_nanos()
    }
}

/// Build one agent configured for runtime tests.
fn agent_for_options(options: &RuntimeOptions) -> (Agent, Host) {
    agent_for_options_with_host_clock_source(options, None)
}

/// Build one agent configured for runtime tests and one optional host clock source.
fn agent_for_options_with_host_clock_source(
    options: &RuntimeOptions,
    host_clock_source: Option<Arc<dyn HostClockSource>>,
) -> (Agent, Host) {
    // construct one runtime agent from explicit options
    let mut agent = if let Some(host_clock_source) = host_clock_source {
        Agent::new_with_host_clock_source(Vec::new(), options, host_clock_source)
            .expect("runtime test agent should build with host clock source")
    } else {
        Agent::new(Vec::new(), options).expect("runtime test agent should build")
    };

    // configure scheduler options for deterministic tests
    agent
        .event_loop
        .configure(options.scheduler.clone())
        .expect("scheduler options should configure");

    // apply runtime options to binding policy state
    agent.bindings.apply_runtime_defaults(options);

    // build the host for this test agent
    let host = Host::from_runtime_options(options);

    // drain initial host bootstrap events for deterministic scheduler tests
    host.poll_events(Some(0))
        .expect("host bootstrap events should drain");

    (agent, host)
}

/// Build one void runtime output.
fn void_output() -> EngineOutput {
    EngineOutput {
        value: heap::Value::VOID,
        telemetry: Default::default(),
        heap_cells: 0,
        raw_heap_cells: 0,
    }
}

/// Convert one test continuation identifier into one native continuation handle.
fn continuation_handle(value: u64) -> usize {
    usize::try_from(value).expect("test continuation id should fit usize")
}
