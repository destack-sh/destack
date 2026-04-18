use std::sync::Arc;

use destack_core::{Capture, CaptureMode};
use destack_engine::Continuation;
use destack_workspace::{RuntimeOptions, SchedulerOptions, TimeMode, TimeOptions};
use {destack_heap as heap, destack_vm as vm};

use crate::diagnostic::RuntimeResult;
use crate::host::{HostEventKind, HostLifecycleState, Session};
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::engine::{
    Engine, EngineImage, EngineLayout, Entry, ExecutionOutcome, ExecutionOutput, LiveContinuation,
    NativeContinuationHandle,
};
use crate::runtime::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::runtime::scheduler::{
    EventLoop, Microtask, MicrotaskId, Runnable, Task, TaskId, TaskStatus, Timer, TimerDeadline,
};
use crate::runtime::time::{Nanos, WorldInstant, host as host_time};
use crate::runtime::{BindingCallContext, DropReason, TickOutcome, Worker, World};

use super::tests::{
    ScriptedHostClockSource, TestEngine, TestMultiAgentRuntime, TestPoller, TestRuntime,
    continuation_from_image, native_continuation_image, validate_native_capture_mode,
};

/// Engine that always completes on resume for microtask tests.
#[derive(Debug, Clone, Default)]
struct CompleteEngine {
    /// Number of resume calls observed.
    resume_calls: usize,
    /// Native continuation identifiers resumed in order.
    resumed_native_ids: Vec<usize>,
}

impl Engine for CompleteEngine {
    /// Run one entrypoint without yielding.
    fn run(
        &mut self,
        _memory: &mut vm::MemoryContext<'_>,
        _entry: &Entry,
        _args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
        Ok(ExecutionOutcome::Completed {
            output: ExecutionOutput {
                value: heap::Value::VOID,
                stats: Default::default(),
                managed_allocation_count: 0,
                raw_allocation_count: 0,
            },
        })
    }

    /// Resume one continuation and complete immediately.
    fn resume(
        &mut self,
        _memory: &mut vm::MemoryContext<'_>,
        continuation: LiveContinuation,
        _value: heap::Value,
    ) -> RuntimeResult<ExecutionOutcome<LiveContinuation>> {
        // record native continuation ids for ordering assertions
        if let LiveContinuation::Native(continuation) = continuation {
            self.resumed_native_ids.push(continuation.get());
        }

        self.resume_calls = self.resume_calls.saturating_add(1);
        Ok(ExecutionOutcome::Completed {
            output: ExecutionOutput {
                value: heap::Value::VOID,
                stats: Default::default(),
                managed_allocation_count: 0,
                raw_allocation_count: 0,
            },
        })
    }

    /// Capture one immutable engine image for scheduler tests.
    fn image(&mut self) -> RuntimeResult<EngineImage> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "scheduler test engine images are not implemented".to_string(),
        }
        .boxed())
    }

    /// Fork one live scheduler test engine.
    fn fork(&mut self, _heap: &mut heap::Heap) -> RuntimeResult<Box<dyn Engine>> {
        Ok(Box::new(self.clone()))
    }

    /// Restore one immutable engine image for scheduler tests.
    fn restore_image(&mut self, _heap: &mut heap::Heap, image: &EngineImage) -> RuntimeResult<()> {
        let _ = image;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "scheduler test engine image restore is not implemented".to_string(),
        }
        .boxed())
    }

    /// Capture one continuation image for scheduler tests.
    fn continuation_image(
        &mut self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
    ) -> RuntimeResult<Continuation> {
        validate_native_capture_mode(continuation, mode)?;

        match continuation {
            LiveContinuation::Native(continuation) => Ok(native_continuation_image(*continuation)),
            LiveContinuation::Vm(_) => Err(crate::diagnostic::RuntimeError::Internal {
                message: "scheduler test engine vm continuation images are not implemented"
                    .to_string(),
            }
            .boxed()),
        }
    }

    /// Restore one continuation image for scheduler tests.
    fn restore_continuation_image(
        &mut self,
        image: &Continuation,
    ) -> RuntimeResult<LiveContinuation> {
        Ok(LiveContinuation::Native(continuation_from_image(image)))
    }

    /// Capture one serialized engine image for scheduler tests.
    fn snapshot(&mut self) -> RuntimeResult<EngineImage> {
        Err(crate::diagnostic::RuntimeError::Internal {
            message: "scheduler test engine snapshots are not implemented".to_string(),
        }
        .boxed())
    }

    /// Restore one serialized engine image for scheduler tests.
    fn restore_snapshot(
        &mut self,
        _heap: &mut heap::Heap,
        snapshot: &EngineImage,
    ) -> RuntimeResult<()> {
        let _ = snapshot;

        Err(crate::diagnostic::RuntimeError::Internal {
            message: "scheduler test engine snapshot restore is not implemented".to_string(),
        }
        .boxed())
    }
}

impl EngineLayout for CompleteEngine {
    /// Return the managed-reference width required by this scheduler test engine.
    fn managed_reference_bytes(&self) -> u8 {
        8
    }
}

/// Executes one queued task in one tick.
#[test]
fn test_tick_executes_one_task() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::new();
    runtime.enqueue_task_native(7, 1, 0);

    // execute one tick and verify one resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    runtime.with_engine::<TestEngine, _>(|engine| {
        assert_eq!(engine.resume_calls, 1, "one task should be resumed once");
    });
}

/// Drains queued tasks and re-yields until idle.
#[test]
fn test_tick_until_idle_drains_yielded_tasks() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::new();
    runtime.enqueue_task_native(11, 9, 0);

    // run ticks until the queue is drained
    runtime.tick_until_idle();

    // verify the yielded continuation was resumed and then completed
    runtime.with_engine::<TestEngine, _>(|engine| {
        assert_eq!(engine.resume_calls, 2, "yielded task should resume twice");
    });
    assert!(
        !runtime.has_pending_work(),
        "event loop should be idle after draining tasks"
    );
}

/// Dispatches registered timer watches through the runtime tick path.
#[test]
fn test_tick_dispatches_timer_watch_task() {
    // create runtime state with one timer watch registration
    let mut runtime = TestRuntime::new();
    runtime.watch_timer_native(77, 31, 0);
    runtime.schedule_timer(77, 0, None);

    // execute one tick and verify one watched resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    runtime.with_engine::<TestEngine, _>(|engine| {
        assert_eq!(engine.resume_calls, 1, "one timer watch should run");
    });

    // one-shot watch should be removed after the first dispatch
    assert!(
        !runtime.unwatch_timer(77),
        "one-shot timer watch should be removed"
    );
}

/// Dispatches registered external events through the runtime tick path.
#[test]
fn test_tick_dispatches_event_watch_task() {
    // create runtime state with one event watch registration
    let mut runtime = TestRuntime::new();
    runtime.watch_event_native(91, 41, 0);
    runtime.enqueue_io_event(5, 91, 9);

    // execute one tick and verify one watched resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    runtime.with_engine::<TestEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "one external event watch should run"
        );
    });
}

/// Dispatches registered host semantic events through the runtime tick path.
#[test]
fn test_tick_dispatches_host_event_watch_task() {
    // create runtime state with one lifecycle host-event watch registration
    let mut runtime = TestRuntime::new();
    runtime.watch_host_event_native(HostEventKind::Lifecycle, 42, 0);
    runtime.enqueue_lifecycle_host_event(HostLifecycleState::Running);

    // execute one tick and verify one watched resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    runtime.with_engine::<TestEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "one host semantic event watch should run"
        );
    });
}

/// Routes event watches into the task queue and preserves priority ordering.
#[test]
fn test_tick_routes_event_watch_through_task_priority() {
    // create runtime state with one queued high-priority task
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), CompleteEngine::default());
    runtime.enqueue_task_native(301, 91, 200);

    // register one low-priority event watch and enqueue one event
    runtime.watch_event_native(44, 92, 0);
    runtime.enqueue_io_event(7, 44, 1);

    // run one tick and verify high-priority task runs before watched event task
    let _ = runtime.tick();
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(engine.resumed_native_ids, vec![91]);
    });

    let _ = runtime.tick();
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(engine.resumed_native_ids, vec![91, 92]);
    });
}

/// Drops queued events that have no registered dispatch watch.
#[test]
fn test_tick_drops_event_without_watch() {
    // create runtime state with one unregistered event token
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), CompleteEngine::default());
    runtime.enqueue_io_event(8, 404, 2);

    // executing one tick should drop the stale event without crashing
    let progressed = runtime.tick();
    assert!(
        progressed,
        "dropping one queued event should count as progress"
    );
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 0,
            "dropped events must not resume any continuation"
        );
    });
    assert_eq!(
        runtime.drop_counts().total(),
        1,
        "dropped external events should be counted for observability"
    );
    assert_eq!(
        runtime.drop_counts().count(DropReason::UnwatchedDispatch),
        1,
        "unwatched dropped events should be tracked separately"
    );
    assert_eq!(
        runtime.drop_counts().count(DropReason::QueuePressure),
        0,
        "host queue pressure should not be counted in this case"
    );
}

/// Drops queued host semantic events that have no registered dispatch watch.
#[test]
fn test_tick_drops_host_event_without_watch() {
    // create runtime state with one unregistered lifecycle host event
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), CompleteEngine::default());
    runtime.enqueue_lifecycle_host_event(HostLifecycleState::Running);

    // executing one tick should drop the stale event without crashing
    let progressed = runtime.tick();
    assert!(
        progressed,
        "dropping one queued host event should count as progress"
    );
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 0,
            "dropped host events must not resume any continuation"
        );
    });
    assert_eq!(
        runtime.drop_counts().total(),
        1,
        "dropped host events should be counted for observability"
    );
    assert_eq!(
        runtime.drop_counts().count(DropReason::UnwatchedDispatch),
        1,
        "unwatched dropped host events should be tracked separately"
    );
    assert_eq!(
        runtime.drop_counts().count(DropReason::QueuePressure),
        0,
        "host queue pressure should not be counted in this case"
    );
}

/// Records unmatched runtime ingress explicitly instead of rerouting it.
#[test]
fn test_runtime_tick_records_unmatched_poller_ingress() {
    // create one multi-worker runtime with no poller watches
    let mut runtime = TestMultiAgentRuntime::with_options_and_engine(
        &RuntimeOptions::default(),
        CompleteEngine::default(),
    );
    runtime.set_poller(Box::new(TestPoller::with_events(vec![PollerEvent {
        resource_id: ResourceId(19),
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(444),
        payload: PollerEventPayload::Io { data: 7 },
    }])));

    // one runtime tick should account for the unmatched ingress without resuming work
    let outcome = runtime.tick();

    assert_eq!(outcome, TickOutcome::Progressed);
    runtime.with_primary_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 0,
            "unmatched ingress must not resume work"
        );
    });
    assert_eq!(
        runtime.drop_counts().count(DropReason::UnmatchedIngress),
        1,
        "unmatched ingress should be counted explicitly"
    );
    assert_eq!(
        runtime.drop_counts().count(DropReason::QueuePressure),
        0,
        "queue pressure should stay unchanged in this case"
    );
}

/// Stops draining when one microtask budget is configured.
#[test]
fn test_tick_respects_microtask_budget() {
    // configure one microtask budget of one
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), CompleteEngine::default());
    runtime.configure_scheduler(SchedulerOptions {
        microtask_budget: Some(1),
        ..SchedulerOptions::default()
    });

    // enqueue two microtasks and run one tick
    runtime.enqueue_microtask_native(1, 51);
    runtime.enqueue_microtask_native(2, 52);

    let _ = runtime.tick();
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "one microtask should run in one tick"
        );
    });

    // one microtask should remain queued for the next tick
    assert!(
        runtime.has_microtasks(),
        "one microtask should remain after budgeted drain"
    );
}

/// Does not treat sequential microtasks as nested depth.
#[test]
fn test_max_microtask_depth_allows_sequential_microtasks() {
    // configure one depth limit of one with budget for two microtasks
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), CompleteEngine::default());
    runtime.configure_scheduler(SchedulerOptions {
        microtask_budget: Some(2),
        max_microtask_depth: Some(1),
        ..SchedulerOptions::default()
    });

    // enqueue two sequential microtasks
    runtime.enqueue_microtask_native(3, 53);
    runtime.enqueue_microtask_native(4, 54);

    // both microtasks should run in one tick without depth failure
    let progressed = runtime.tick();
    assert!(progressed, "tick should execute queued microtasks");
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(engine.resume_calls, 2, "both microtasks should complete");
    });
}

/// Dequeues microtasks before macrotasks.
#[test]
fn test_event_loop_next_runnable_prioritizes_microtasks() {
    // set up an event loop with one task and one microtask
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(501),
        runnable: LiveContinuation::Native(NativeContinuationHandle::new(601)),
        resume_value: heap::Value::VOID,
        status: TaskStatus::Ready,
        priority: 0,
    });
    event_loop.enqueue_microtask(Microtask {
        id: MicrotaskId::new(502),
        continuation: LiveContinuation::Native(NativeContinuationHandle::new(602)),
        resume_value: heap::Value::VOID,
        status: TaskStatus::Ready,
    });

    // verify microtask dispatch precedes task dispatch
    let first = event_loop
        .next_runnable(0, 0)
        .expect("event loop should dequeue runnable");
    assert!(matches!(first, Some(Runnable::Microtask(_))));

    let second = event_loop
        .next_runnable(0, 0)
        .expect("event loop should dequeue runnable");
    assert!(matches!(second, Some(Runnable::Task(_))));
}

/// Orders queued tasks by descending priority.
#[test]
fn test_event_loop_next_runnable_prioritizes_higher_task_priority() {
    // set up an event loop with low and high priority tasks
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(503),
        runnable: LiveContinuation::Native(NativeContinuationHandle::new(603)),
        resume_value: heap::Value::VOID,
        status: TaskStatus::Ready,
        priority: 1,
    });
    event_loop.enqueue_task(Task {
        id: TaskId::new(504),
        runnable: LiveContinuation::Native(NativeContinuationHandle::new(604)),
        resume_value: heap::Value::VOID,
        status: TaskStatus::Ready,
        priority: 200,
    });

    // verify higher priority task dequeues first
    let first = event_loop
        .next_runnable(0, 0)
        .expect("event loop should dequeue runnable");
    let Some(Runnable::Task(task)) = first else {
        panic!("expected one task runnable");
    };
    assert_eq!(task.id.get(), 504);
}

/// Rejects suspend images for native continuations that cannot be restored honestly.
#[test]
fn test_event_loop_suspend_rejects_native_continuations() {
    // one queued native task
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(601),
        runnable: LiveContinuation::Native(NativeContinuationHandle::new(701)),
        resume_value: heap::Value::VOID,
        status: TaskStatus::Ready,
        priority: 0,
    });

    // suspend capture should fail loudly
    let mut engine = CompleteEngine::default();
    let result = event_loop.capture_image(CaptureMode::Suspend, &mut engine);

    assert!(result.is_err(), "native suspend capture should fail loudly");
}

/// Roundtrips queued scheduler state through one suspend image.
#[test]
fn test_event_loop_suspend_roundtrip_preserves_pending_state() {
    // one queued poller event and one ready timer
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_events(vec![PollerEvent {
        resource_id: ResourceId(61),
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(991),
        payload: PollerEventPayload::Io { data: 7 },
    }]);
    event_loop
        .schedule_timer(Timer {
            handle: ResourceId(62).into(),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("schedule timer");
    event_loop
        .enqueue_due_timers(Nanos::new(0), Nanos::new(0))
        .expect("enqueue ready timers");

    // capture and restore one suspend image
    let mut engine = CompleteEngine::default();
    let image = event_loop
        .capture_image(CaptureMode::Suspend, &mut engine)
        .expect("capture suspend image");
    let mut restored = EventLoop::default();
    restored
        .restore_image(&image, &mut engine)
        .expect("restore suspend image");

    // ready timer stays ahead of queued poller events
    let first = restored
        .next_runnable(0, 0)
        .expect("dequeue first runnable");
    assert!(matches!(first, Some(Runnable::Timer(_))));

    let second = restored
        .next_runnable(0, 0)
        .expect("dequeue second runnable");
    assert!(matches!(second, Some(Runnable::PollerEvent(_))));
}

/// Rejects non-fifo scheduler policies.
#[test]
fn test_event_loop_configure_rejects_non_fifo_policy() {
    // configure one unsupported fair scheduler policy
    let mut event_loop = EventLoop::default();
    let options = SchedulerOptions {
        policy: destack_workspace::SchedulerPolicy::Fair,
        ..SchedulerOptions::default()
    };

    // verify configure fails loudly
    let result = event_loop.configure(options);
    assert!(result.is_err(), "non-fifo policy should fail");
}

/// Drops ready timers that were canceled before dispatch.
#[test]
fn test_event_loop_cancel_timer_drops_ready_timer_before_dispatch() {
    // enqueue one timer and promote it into the ready queue
    let mut event_loop = EventLoop::default();
    event_loop
        .schedule_timer(Timer {
            handle: ResourceId(700).into(),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("timer should schedule");
    event_loop
        .enqueue_due_timers(Nanos::new(0), Nanos::new(0))
        .expect("ready timers should enqueue");

    // cancel before dequeue and verify dispatch is suppressed
    event_loop
        .cancel_timer(ResourceId(700))
        .expect("timer cancel should succeed");
    let next = event_loop
        .next_runnable(0, 0)
        .expect("event loop should dequeue runnable");
    assert!(
        next.is_none(),
        "canceled ready timer should not dispatch as runnable work"
    );
}

/// Returns idle in virtual time when waiting work is pending but not ready.
#[test]
fn test_run_loop_until_task_complete_returns_idle_for_virtual_time_waits() {
    // build one runtime in virtual-time mode
    let options = RuntimeOptions {
        time: destack_workspace::TimeOptions {
            mode: TimeMode::Virtual,
            ..destack_workspace::TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime = TestRuntime::with_options_and_engine(&options, CompleteEngine::default());

    // enqueue one timer that is not yet ready
    runtime.schedule_timer(900, 1_000_000, None);

    // running for one nonexistent target task should return idle instead of spinning
    let error = runtime
        .run_loop_until_task_complete(12345)
        .expect_err("virtual mode should not block or spin to advance time");
    assert!(
        error
            .message()
            .contains("event loop idle before completing task 12345"),
        "virtual mode should report idle when pending work is not ready"
    );
}

/// Returns none when one timeout elapses before the target task completes.
#[test]
fn test_run_loop_until_task_complete_with_timeout_returns_none() {
    // configure one scripted host clock source for deterministic host mode waits
    let host_clock_source = Arc::new(ScriptedHostClockSource::new(1_000_000, 0));
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Host,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        CompleteEngine::default(),
        host_clock_source.clone(),
    );

    // schedule one timer later than the configured timeout
    let fire_at_nanos = runtime.wall_nanos().saturating_add(50_000_000);
    runtime.watch_timer_native(910, 77, 0);
    runtime.schedule_timer(910, fire_at_nanos, None);

    // timeout should elapse before one watched task can complete
    let output = runtime
        .run_loop_until_task_complete_with_timeout(0, Some(1_000_000))
        .expect("bounded run loop should return timeout result");
    assert!(output.is_none(), "timeout should return no output");
    assert!(
        host_clock_source.mono_nanos() >= 1_000_000,
        "host wait should advance scripted monotonic time"
    );
}

/// Waits in host mode until one future timer dispatches the target task.
#[test]
fn test_run_loop_until_task_complete_waits_for_host_timer() {
    // configure one scripted host clock source for deterministic host mode waits
    let host_clock_source = Arc::new(ScriptedHostClockSource::new(2_000_000, 0));
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Host,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        CompleteEngine::default(),
        host_clock_source.clone(),
    );

    // schedule one near-future timer for the first watched task id
    let fire_at_nanos = runtime.wall_nanos().saturating_add(5_000_000);
    runtime.watch_timer_native(920, 88, 0);
    runtime.schedule_timer_on(TimerClock::Wall, 920, fire_at_nanos, None);

    // host-mode run loop should wait and complete the target task
    let output = runtime
        .run_loop_until_task_complete(0)
        .expect("host mode should wait for the timer and complete the task");
    assert_eq!(output.value, heap::Value::VOID);
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "one watched timer task should resume once"
        );
    });
    assert!(
        host_clock_source.wall_nanos() >= fire_at_nanos,
        "host wait should advance scripted wall time to the timer deadline"
    );
}

/// Dispatches wall timers after one wall-clock jump without monotonic advancement.
#[test]
fn test_wall_clock_jump_fires_wall_timer() {
    // configure one host-mode runtime with one scripted clock source
    let host_clock_source = Arc::new(ScriptedHostClockSource::new(10_000, 500));
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Host,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        CompleteEngine::default(),
        host_clock_source.clone(),
    );

    // schedule one wall timer and jump wall time beyond the deadline
    let fire_at_nanos = runtime.wall_nanos().saturating_add(1_000);
    runtime.watch_timer_native(930, 97, 0);
    runtime.schedule_timer_on(TimerClock::Wall, 930, fire_at_nanos, None);
    host_clock_source.jump_wall_nanos(2_000);

    // one tick should dispatch the watched timer task
    let progressed = runtime.tick();
    assert!(progressed, "wall jump should make wall timer ready");
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "wall timer should dispatch after wall jump"
        );
    });
    assert_eq!(
        runtime.mono_nanos(),
        500,
        "wall jump should not implicitly advance monotonic time"
    );
}

/// Keeps monotonic timers pending across wall-clock jumps.
#[test]
fn test_wall_clock_jump_does_not_fire_monotonic_timer() {
    // configure one host-mode runtime with one scripted clock source
    let host_clock_source = Arc::new(ScriptedHostClockSource::new(20_000, 900));
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Host,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        CompleteEngine::default(),
        host_clock_source.clone(),
    );

    // schedule one monotonic timer and jump wall time only
    let fire_at_nanos = runtime.mono_nanos().saturating_add(1_000);
    runtime.watch_timer_native(940, 98, 0);
    runtime.schedule_timer_on(TimerClock::Monotonic, 940, fire_at_nanos, None);
    host_clock_source.jump_wall_nanos(10_000);

    // wall jumps must not dispatch monotonic timers
    let progressed = runtime.tick();
    assert!(
        !progressed,
        "monotonic timer should remain pending after a wall jump"
    );
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 0,
            "monotonic timer should not dispatch before monotonic time advances"
        );
    });

    // monotonic advancement should make the timer ready
    host_clock_source.advance_mono_nanos(1_000);
    let progressed = runtime.tick();
    assert!(
        progressed,
        "monotonic timer should dispatch once monotonic time advances"
    );
    runtime.with_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "monotonic timer should dispatch exactly once"
        );
    });
}

/// Advances virtual time to the next deadline before dispatching the timer task.
#[test]
fn test_runtime_tick_advances_virtual_time_before_dispatch() {
    // configure one virtual runtime with one future timer
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Virtual,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime =
        TestMultiAgentRuntime::with_options_and_engine(&options, CompleteEngine::default());
    let primary_worker_id = runtime.primary_worker_id();
    let fire_at_nanos = runtime.wall_nanos().saturating_add(5_000);
    runtime.with_worker_mut(primary_worker_id, |worker| {
        register_timer_watch(worker, 950, 111, 0);
        schedule_timer(worker, TimerClock::Wall, 950, fire_at_nanos, None);
    });

    // the first tick should only advance time
    let outcome = runtime.tick();
    assert_eq!(outcome, TickOutcome::AdvancedTime);
    runtime.with_primary_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 0,
            "deadline jump must not run work yet"
        );
    });
    assert_eq!(
        runtime.wall_nanos(),
        fire_at_nanos,
        "virtual time should jump exactly to the next deadline"
    );
    assert_eq!(
        runtime.mono_nanos(),
        fire_at_nanos,
        "virtual monotonic time should advance with wall time"
    );

    // the next tick should dispatch the newly ready timer task
    let outcome = runtime.tick();
    assert_eq!(outcome, TickOutcome::Progressed);
    runtime.with_primary_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 1,
            "timer watch should run after the jump"
        );
    });
}

/// Executes one world tick through the attached runtime.
#[test]
fn test_world_tick_drives_runtime() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("world");
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, CompleteEngine::default())
        .expect("runtime should spawn");
    // enqueue one native task on the primary worker
    let runtime = world.runtime_mut(runtime_id).expect("runtime should exist");
    let primary_worker_id = runtime.primary_worker_id();
    let worker = runtime
        .worker_mut(primary_worker_id)
        .expect("primary worker");
    worker.event_loop.enqueue_task(Task {
        id: TaskId::new(1),
        runnable: LiveContinuation::Native(NativeContinuationHandle::new(continuation_handle(211))),
        resume_value: heap::Value::VOID,
        status: TaskStatus::Ready,
        priority: 0,
    });

    // world tick should delegate through the runtime and execute the task
    assert_eq!(world.tick().expect("world tick"), TickOutcome::Progressed);
    let runtime = world.runtime(runtime_id).expect("runtime should exist");
    let worker = runtime.worker(primary_worker_id).expect("primary worker");
    let engine = worker.engine.as_ref() as &dyn std::any::Any;
    let engine = engine
        .downcast_ref::<CompleteEngine>()
        .expect("runtime engine should exist");
    assert_eq!(engine.resume_calls, 1);
}

/// Dispatches equal-deadline timers in stable worker-id order.
#[test]
fn test_runtime_tick_orders_equal_deadline_timers_by_worker_id() {
    // configure one virtual runtime with two workers and one equal deadline
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Virtual,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut runtime =
        TestMultiAgentRuntime::with_options_and_engine(&options, CompleteEngine::default());
    let primary_worker_id = runtime.primary_worker_id();
    let secondary_worker_id = runtime.spawn_worker(CompleteEngine::default());
    let fire_at_nanos = runtime.wall_nanos().saturating_add(10_000);

    // register one watched timer on each worker
    runtime.with_worker_mut(primary_worker_id, |worker| {
        register_timer_watch(worker, 960, 201, 0);
        schedule_timer(worker, TimerClock::Wall, 960, fire_at_nanos, None);
    });
    runtime.with_worker_mut(secondary_worker_id, |worker| {
        register_timer_watch(worker, 961, 202, 0);
        schedule_timer(worker, TimerClock::Wall, 961, fire_at_nanos, None);
    });

    // the first tick advances time and later ticks dispatch in worker order
    assert_eq!(runtime.tick(), TickOutcome::AdvancedTime);
    assert_eq!(runtime.tick(), TickOutcome::Progressed);
    runtime.with_worker_engine::<CompleteEngine, _>(primary_worker_id, |engine| {
        assert_eq!(engine.resumed_native_ids, vec![201]);
    });
    assert_eq!(runtime.tick(), TickOutcome::Progressed);
    runtime.with_worker_engine::<CompleteEngine, _>(primary_worker_id, |engine| {
        assert_eq!(engine.resumed_native_ids, vec![201]);
    });
    runtime.with_worker_engine::<CompleteEngine, _>(secondary_worker_id, |engine| {
        assert_eq!(engine.resumed_native_ids, vec![202]);
    });
    assert_eq!(runtime.tick(), TickOutcome::Idle);
}

/// Advances virtual time to one simulation deadline when no worker work is ready.
#[test]
fn test_runtime_tick_advances_to_simulation_deadline() {
    // configure one virtual runtime with one simulated wakeup
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Virtual,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let runtime =
        TestMultiAgentRuntime::with_options_and_engine(&options, CompleteEngine::default());
    let mut runtime = runtime;
    runtime
        .world_mut()
        .simulation_mut()
        .schedule_event(WorldInstant::new(7_500));

    // the first tick should advance world time to the simulated deadline
    let outcome = runtime.tick();
    assert_eq!(outcome, TickOutcome::AdvancedTime);
    assert_eq!(runtime.wall_nanos(), 7_500);
    assert_eq!(runtime.mono_nanos(), 7_500);
    let world = runtime.world();
    let simulation = world.simulation();
    assert_eq!(simulation.ready_events().len(), 1);
    assert_eq!(simulation.ready_events()[0].at(), WorldInstant::new(7_500));
    runtime.with_primary_engine::<CompleteEngine, _>(|engine| {
        assert_eq!(
            engine.resume_calls, 0,
            "simulation deadline should not run worker work"
        );
    });
}

/// Rejects synchronous virtual sleeps so time only advances through runtime ticks.
#[test]
fn test_virtual_sleep_binding_fails_loudly() {
    // build one virtual-time binding context
    let options = RuntimeOptions {
        time: TimeOptions {
            mode: TimeMode::Virtual,
            ..TimeOptions::default()
        },
        ..RuntimeOptions::default()
    };
    let mut world = World::from_options(&options).expect("world");
    let world_ref = world.world_ref();
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        Box::new(TestEngine::default()),
    )
    .expect("worker should build");
    let host = Session::from_runtime_options(&options, worker.runtime_id);
    let binding = BindingCallContext::new(&worker, worker.event_loop.as_ref(), &host, &world_ref);
    let wall_before = binding.wall_nanos();

    // synchronous sleep must fail instead of advancing virtual time inline
    let error = unsafe { host_time::host_sleep_nanos(&binding, 123) }
        .expect_err("virtual sleep should fail");
    assert!(
        error.message().contains("destack.time.sleep.ns"),
        "virtual sleep error should name the binding"
    );
    assert_eq!(
        binding.wall_nanos(),
        wall_before,
        "virtual sleep must not advance world time directly"
    );
}

/// Register one native timer watch on one explicit worker.
fn register_timer_watch(worker: &mut Worker, handle: u64, continuation_id: u64, priority: u8) {
    worker
        .watch_timer(
            ResourceId(handle),
            LiveContinuation::Native(NativeContinuationHandle::new(continuation_handle(
                continuation_id,
            ))),
            heap::Value::VOID,
            priority,
        )
        .expect("timer watch should register");
}

/// Schedule one timer on one explicit worker.
fn schedule_timer(
    worker: &mut Worker,
    clock: TimerClock,
    handle: u64,
    fire_at_nanos: u64,
    interval_nanos: Option<u64>,
) {
    worker
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

/// Convert one test continuation identifier into one native continuation handle.
fn continuation_handle(value: u64) -> usize {
    usize::try_from(value).expect("test continuation id should fit usize")
}
