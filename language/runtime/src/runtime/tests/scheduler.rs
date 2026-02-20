use std::sync::Arc;

use destack_vm as vm;
use destack_workspace::{RuntimeOptions, SchedulerOptions, TimeMode, TimeOptions};

use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, NativeContinuation, RuntimeOutput, RuntimeValue,
};
use crate::runtime::scheduler::{
    EventLoop, Microtask, MicrotaskId, Runnable, Task, TaskId, TaskStatus, Timer,
};

use super::tests::{ScriptedHostClockSource, TestEngine, TestRuntime};

/// Engine that always completes on resume for microtask tests.
#[derive(Debug, Default)]
struct CompleteEngine {
    /// Number of resume calls observed.
    resume_calls: usize,
    /// Native continuation identifiers resumed in order.
    resumed_native_ids: Vec<u64>,
}

impl Engine for CompleteEngine {
    /// Entrypoint payload.
    type Entry = ();
    /// Runtime output payload.
    type Output = RuntimeOutput;
    /// Runtime value payload.
    type Value = RuntimeValue;

    /// Run one entrypoint without yielding.
    fn run(
        &mut self,
        _entry: &Self::Entry,
        _args: &[Self::Value],
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Value>> {
        Ok(EngineOutcome::Completed {
            output: RuntimeOutput {
                value: RuntimeValue::VOID,
                statistics: vm::telemetry::Statistics::default(),
                heap_cells: 0,
                raw_heap_cells: 0,
            },
        })
    }

    /// Resume one continuation and complete immediately.
    fn resume(
        &mut self,
        continuation: EngineContinuation,
        _value: Self::Value,
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Value>> {
        // record native continuation ids for ordering assertions
        if let EngineContinuation::Native(continuation) = continuation {
            self.resumed_native_ids.push(continuation.get());
        }

        self.resume_calls = self.resume_calls.saturating_add(1);
        Ok(EngineOutcome::Completed {
            output: RuntimeOutput {
                value: RuntimeValue::VOID,
                statistics: vm::telemetry::Statistics::default(),
                heap_cells: 0,
                raw_heap_cells: 0,
            },
        })
    }
}

/// Executes one queued task in one tick.
#[test]
fn test_tick_once_executes_one_task() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::new();
    runtime.enqueue_task_native(7, 1, 0);

    // execute one tick and verify one resume
    let mut engine = TestEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(progressed, "tick should report progress");
    assert_eq!(engine.resume_calls, 1, "one task should be resumed once");
}

/// Drains queued tasks and re-yields until idle.
#[test]
fn test_tick_until_idle_drains_yielded_tasks() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::new();
    runtime.enqueue_task_native(11, 9, 0);

    // run ticks until the queue is drained
    let mut engine = TestEngine::default();
    runtime.tick_until_idle(&mut engine);

    // verify the yielded continuation was resumed and then completed
    assert_eq!(engine.resume_calls, 2, "yielded task should resume twice");
    assert!(
        !runtime.has_pending_work(),
        "event loop should be idle after draining tasks"
    );
}

/// Dispatches registered timer watches through the runtime tick path.
#[test]
fn test_tick_once_dispatches_timer_watch_task() {
    // create runtime state with one timer watch registration
    let mut runtime = TestRuntime::new();
    runtime.watch_timer_native(77, 31, 0);
    runtime.schedule_timer(77, 0, None);

    // execute one tick and verify one watched resume
    let mut engine = TestEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(progressed, "tick should report progress");
    assert_eq!(engine.resume_calls, 1, "one timer watch should run");

    // one-shot watch should be removed after the first dispatch
    assert!(
        !runtime.unwatch_timer(77),
        "one-shot timer watch should be removed"
    );
}

/// Dispatches registered external events through the runtime tick path.
#[test]
fn test_tick_once_dispatches_event_watch_task() {
    // create runtime state with one event watch registration
    let mut runtime = TestRuntime::new();
    runtime.watch_event_native(91, 41, 0);
    runtime.enqueue_io_event(5, 91, 9);

    // execute one tick and verify one watched resume
    let mut engine = TestEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(progressed, "tick should report progress");
    assert_eq!(
        engine.resume_calls, 1,
        "one external event watch should run"
    );
}

/// Routes event watches into the task queue and preserves priority ordering.
#[test]
fn test_tick_once_routes_event_watch_through_task_priority() {
    // create runtime state with one queued high-priority task
    let mut runtime = TestRuntime::new();
    runtime.enqueue_task_native(301, 91, 200);

    // register one low-priority event watch and enqueue one event
    runtime.watch_event_native(44, 92, 0);
    runtime.enqueue_io_event(7, 44, 1);

    // run one tick and verify high-priority task runs before watched event task
    let mut engine = CompleteEngine::default();
    let _ = runtime.tick_once(&mut engine);
    assert_eq!(engine.resumed_native_ids, vec![91]);

    let _ = runtime.tick_once(&mut engine);
    assert_eq!(engine.resumed_native_ids, vec![91, 92]);
}

/// Drops queued events that have no registered dispatch watch.
#[test]
fn test_tick_once_drops_event_without_watch() {
    // create runtime state with one unregistered event token
    let mut runtime = TestRuntime::new();
    runtime.enqueue_io_event(8, 404, 2);

    // executing one tick should drop the stale event without crashing
    let mut engine = CompleteEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(
        progressed,
        "dropping one queued event should count as progress"
    );
    assert_eq!(
        engine.resume_calls, 0,
        "dropped events must not resume any continuation"
    );
    assert_eq!(
        runtime.dropped_external_events(),
        1,
        "dropped external events should be counted for observability"
    );
}

/// Stops draining when one microtask budget is configured.
#[test]
fn test_tick_once_respects_microtask_budget() {
    // configure one microtask budget of one
    let mut runtime = TestRuntime::new();
    runtime.configure_scheduler(SchedulerOptions {
        microtask_budget: Some(1),
        ..SchedulerOptions::default()
    });

    // enqueue two microtasks and run one tick
    runtime.enqueue_microtask_native(1, 51);
    runtime.enqueue_microtask_native(2, 52);

    let mut engine = CompleteEngine::default();
    let _ = runtime.tick_once(&mut engine);
    assert_eq!(
        engine.resume_calls, 1,
        "one microtask should run in one tick"
    );

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
    let mut runtime = TestRuntime::new();
    runtime.configure_scheduler(SchedulerOptions {
        microtask_budget: Some(2),
        max_microtask_depth: Some(1),
        ..SchedulerOptions::default()
    });

    // enqueue two sequential microtasks
    runtime.enqueue_microtask_native(3, 53);
    runtime.enqueue_microtask_native(4, 54);

    // both microtasks should run in one tick without depth failure
    let mut engine = CompleteEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(progressed, "tick should execute queued microtasks");
    assert_eq!(engine.resume_calls, 2, "both microtasks should complete");
}

/// Dequeues microtasks before macrotasks.
#[test]
fn test_event_loop_next_runnable_prioritizes_microtasks() {
    // set up an event loop with one task and one microtask
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(501),
        runnable: EngineContinuation::Native(NativeContinuation::new(601)),
        resume_value: RuntimeValue::VOID,
        status: TaskStatus::Ready,
        priority: 0,
    });
    event_loop.enqueue_microtask(Microtask {
        id: MicrotaskId::new(502),
        runnable: EngineContinuation::Native(NativeContinuation::new(602)),
        resume_value: RuntimeValue::VOID,
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
        runnable: EngineContinuation::Native(NativeContinuation::new(603)),
        resume_value: RuntimeValue::VOID,
        status: TaskStatus::Ready,
        priority: 1,
    });
    event_loop.enqueue_task(Task {
        id: TaskId::new(504),
        runnable: EngineContinuation::Native(NativeContinuation::new(604)),
        resume_value: RuntimeValue::VOID,
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
            clock: TimerClock::Wall,
            handle: ResourceId(700),
            fire_at_nanos: 0,
            interval_nanos: None,
        })
        .expect("timer should schedule");
    event_loop
        .enqueue_ready_timers(0, 0)
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
    let mut runtime = TestRuntime::with_options(&options);

    // enqueue one timer that is not yet ready
    runtime.schedule_timer(900, 1_000_000, None);

    // running for one nonexistent target task should return idle instead of spinning
    let mut engine = CompleteEngine::default();
    let error = runtime
        .run_loop_until_task_complete(&mut engine, 12345)
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
    let mut runtime =
        TestRuntime::with_options_and_host_clock_source(&options, host_clock_source.clone());

    // schedule one timer later than the configured timeout
    let fire_at_nanos = runtime.wall_nanos().saturating_add(50_000_000);
    runtime.watch_timer_native(910, 77, 0);
    runtime.schedule_timer(910, fire_at_nanos, None);

    // timeout should elapse before one watched task can complete
    let mut engine = CompleteEngine::default();
    let output = runtime
        .run_loop_until_task_complete_with_timeout(&mut engine, 0, Some(1_000_000))
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
    let mut runtime =
        TestRuntime::with_options_and_host_clock_source(&options, host_clock_source.clone());

    // schedule one near-future timer for the first watched task id
    let fire_at_nanos = runtime.wall_nanos().saturating_add(5_000_000);
    runtime.watch_timer_native(920, 88, 0);
    runtime.schedule_timer_on(TimerClock::Wall, 920, fire_at_nanos, None);

    // host-mode run loop should wait and complete the target task
    let mut engine = CompleteEngine::default();
    let output = runtime
        .run_loop_until_task_complete(&mut engine, 0)
        .expect("host mode should wait for the timer and complete the task");
    assert_eq!(output.value, RuntimeValue::VOID);
    assert_eq!(
        engine.resume_calls, 1,
        "one watched timer task should resume once"
    );
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
    let mut runtime =
        TestRuntime::with_options_and_host_clock_source(&options, host_clock_source.clone());

    // schedule one wall timer and jump wall time beyond the deadline
    let fire_at_nanos = runtime.wall_nanos().saturating_add(1_000);
    runtime.watch_timer_native(930, 97, 0);
    runtime.schedule_timer_on(TimerClock::Wall, 930, fire_at_nanos, None);
    host_clock_source.jump_wall_nanos(2_000);

    // one tick should dispatch the watched timer task
    let mut engine = CompleteEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(progressed, "wall jump should make wall timer ready");
    assert_eq!(
        engine.resume_calls, 1,
        "wall timer should dispatch after wall jump"
    );
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
    let mut runtime =
        TestRuntime::with_options_and_host_clock_source(&options, host_clock_source.clone());

    // schedule one monotonic timer and jump wall time only
    let fire_at_nanos = runtime.mono_nanos().saturating_add(1_000);
    runtime.watch_timer_native(940, 98, 0);
    runtime.schedule_timer_on(TimerClock::Monotonic, 940, fire_at_nanos, None);
    host_clock_source.jump_wall_nanos(10_000);

    // wall jumps must not dispatch monotonic timers
    let mut engine = CompleteEngine::default();
    let progressed = runtime.tick_once(&mut engine);
    assert!(
        !progressed,
        "monotonic timer should remain pending after a wall jump"
    );
    assert_eq!(
        engine.resume_calls, 0,
        "monotonic timer should not dispatch before monotonic time advances"
    );

    // monotonic advancement should make the timer ready
    host_clock_source.advance_mono_nanos(1_000);
    let progressed = runtime.tick_once(&mut engine);
    assert!(
        progressed,
        "monotonic timer should dispatch once monotonic time advances"
    );
    assert_eq!(
        engine.resume_calls, 1,
        "monotonic timer should dispatch exactly once"
    );
}
