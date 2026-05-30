use destack_core::{Capture, CaptureMode};
use destack_engine as engine;
use destack_native as native;
use destack_workspace::{Environment, ExecutionMode, RuntimeOptions};

use crate::host::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::host::time::TimerClock;
use crate::host::{HostEventKind, LifecycleState, ResourceId};
use crate::runtime::engine::{Continuation, Engine};
use crate::runtime::scheduler::{
    EventLoop, Microtask, MicrotaskId, Readiness, ScheduledTimer, Task, TaskId, TimerDeadline, Wake,
};
use crate::runtime::tests::{
    TestEngine, TestRuntime, TestWorldRuntime, start_worker_continuation, test_resource_id,
};
use crate::runtime::time::{Instant, Nanos};
use crate::runtime::{TickResult, Worker};
use crate::world::World;

/// Build runtime options with one explicit execution mode.
fn runtime_options_with_execution(mode: ExecutionMode) -> RuntimeOptions {
    let mut options = RuntimeOptions::default();
    options.execution.mode = mode;

    options
}

/// Executes one queued task in one tick.
#[test]
fn test_tick_executes_one_task() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestEngine::default());
    runtime.enqueue_task_native(7, 1, 0);

    // execute one tick and verify one resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    assert!(
        runtime.has_pending_work(),
        "yielded task should stay queued"
    );
}

/// Drains queued tasks and re-yields until idle.
#[test]
fn test_tick_until_idle_drains_yielded_tasks() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestEngine::default());
    runtime.enqueue_task_native(11, 9, 0);

    // run ticks until the queue is drained
    runtime.tick_until_idle();

    assert!(
        !runtime.has_pending_work(),
        "event loop should be idle after draining tasks"
    );
}

/// Dispatches registered timer waiters through the runtime tick path.
#[test]
fn test_tick_dispatches_timer_waiter_task() {
    // create runtime state with one timer waiter registration
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestEngine::default());
    runtime.add_timer_waiter_native(77, 31, 0);
    runtime.schedule_timer(77, 0, None);

    // execute one tick and verify one waiter resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    assert!(
        runtime.has_pending_work(),
        "yielded timer task should stay queued"
    );

    // one-shot waiter should be removed after the first dispatch
    assert!(
        !runtime.remove_timer_waiter(77),
        "one-shot timer waiter should be removed"
    );
}

/// Dispatches registered resource wakes through the runtime tick path.
#[test]
fn test_tick_dispatches_event_waiter_task() {
    // create runtime state with one resource waiter registration
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestEngine::default());
    runtime.add_resource_waiter_native(5, 41, 0);
    runtime.enqueue_io_event(5, 91, 9);

    // execute one tick and verify one waiter resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    assert!(
        runtime.has_pending_work(),
        "yielded event task should stay queued"
    );
}

/// Dispatches registered host wakes through the runtime tick path.
#[test]
fn test_tick_dispatches_host_event_waiter_task() {
    // create runtime state with one lifecycle host waiter registration
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestEngine::default());
    runtime.add_host_waiter_native(HostEventKind::Lifecycle, 42, 0);
    runtime.enqueue_lifecycle_host_event(LifecycleState::Running);

    // execute one tick and verify one waiter resume
    let progressed = runtime.tick();
    assert!(progressed, "tick should report progress");
    assert!(
        runtime.has_pending_work(),
        "yielded host task should stay queued"
    );
}

/// Dispatches resource waiters through task priority ordering.
#[test]
fn test_tick_dispatches_event_waiter_by_task_priority() {
    // create runtime state with one queued high-priority task
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestEngine::default());
    runtime.enqueue_task_native(301, 91, 200);

    // register one low-priority resource waiter and enqueue one wake
    runtime.add_resource_waiter_native(7, 92, 0);
    runtime.enqueue_io_event(7, 44, 1);

    // run high-priority task before waiter wake work
    let _ = runtime.tick();
    assert!(
        runtime.has_pending_work(),
        "waiter wake should remain after the high-priority task"
    );

    let _ = runtime.tick();
    assert!(
        runtime.has_pending_work(),
        "yielded waiter wake task should stay queued"
    );
}

/// Dequeues microtasks before macrotasks.
#[test]
fn test_event_loop_drains_microtasks_before_tasks() {
    // set up an event loop with one task and one microtask
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(501),
        runnable: native_continuation(601),
        resume_value: engine::Value::Void,
        priority: 0,
    });
    event_loop.enqueue_microtask(Microtask {
        id: MicrotaskId::new(502),
        continuation: native_continuation(602),
        resume_value: engine::Value::Void,
    });

    // verify microtask dispatch precedes task dispatch
    let first = event_loop.pop_microtask();
    assert!(first.is_some(), "microtask should dequeue first");

    let second = event_loop.pop_task();
    assert!(second.is_some(), "task should dequeue after microtasks");
}

/// Orders queued tasks by descending priority.
#[test]
fn test_event_loop_pop_task_prioritizes_higher_task_priority() {
    // set up an event loop with low and high priority tasks
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(503),
        runnable: native_continuation(603),
        resume_value: engine::Value::Void,
        priority: 1,
    });
    event_loop.enqueue_task(Task {
        id: TaskId::new(504),
        runnable: native_continuation(604),
        resume_value: engine::Value::Void,
        priority: 200,
    });

    // verify higher priority task dequeues first
    let task = event_loop.pop_task().expect("task should dequeue");
    assert_eq!(task.id.get(), 504);
}

/// Rejects suspend images for native continuations that cannot be restored honestly.
#[test]
fn test_event_loop_suspend_rejects_native_continuations() {
    // one queued native task
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(601),
        runnable: native_continuation(701),
        resume_value: engine::Value::Void,
        priority: 0,
    });

    // suspend capture should fail loudly
    let mut engine = Engine::from(TestEngine::default());
    let result = event_loop.capture_image(CaptureMode::Suspend, &mut engine);

    assert!(result.is_err(), "native suspend capture should fail loudly");
}

/// Roundtrips queued scheduler state through one suspend image.
#[test]
fn test_event_loop_suspend_roundtrip_preserves_pending_state() {
    // one queued resource wake and one ready timer
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_poller_wakes(vec![PollerEvent {
        resource_id: test_resource_id(61),
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(991),
        payload: PollerEventPayload::Io { data: 7 },
    }]);
    event_loop
        .schedule_timer(ScheduledTimer {
            resource_id: test_resource_id(62),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("schedule timer");

    // capture and restore one suspend image
    let mut engine = Engine::from(TestEngine::default());
    let image = event_loop
        .capture_image(CaptureMode::Suspend, &mut engine)
        .expect("capture suspend image");
    let mut restored = EventLoop::default();
    restored
        .restore_image(&image, &mut engine)
        .expect("restore suspend image");

    // ready timer stays ahead of queued resource wakes
    let first = restored
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("dequeue first wake");
    assert!(matches!(first, Some(Wake::Timer(_))));

    let second = restored
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("dequeue second wake");
    assert!(matches!(
        second,
        Some(Wake::Resource(wake)) if wake.readiness == Readiness::Readable
    ));
}

/// Drops ready timers that were canceled before dispatch.
#[test]
fn test_event_loop_cancel_timer_drops_ready_timer_before_dispatch() {
    // enqueue one timer and cancel it before dispatch
    let mut event_loop = EventLoop::default();
    event_loop
        .schedule_timer(ScheduledTimer {
            resource_id: test_resource_id(700),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("timer should schedule");
    // cancel before dequeue and verify dispatch is suppressed
    event_loop
        .cancel_timer(test_resource_id(700))
        .expect("timer cancel should succeed");
    let next = event_loop
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("event loop should dequeue wake");
    assert!(
        next.is_none(),
        "canceled ready timer should not dispatch as runnable work"
    );
}

/// Returns idle in virtual time when waiting work is pending but not ready.
#[test]
fn test_run_loop_until_task_complete_returns_idle_for_virtual_time_waits() {
    // build one runtime with a runtime-owned clock source
    let options = runtime_options_with_execution(ExecutionMode::Strict);
    let mut runtime = TestRuntime::build(&options, TestEngine::default());

    // enqueue one timer that is not yet ready
    runtime.schedule_timer(900, 1_000_000, None);

    // running for one nonexistent target task should return idle instead of spinning
    let output = runtime
        .run_loop_until_task_complete(12345, None)
        .expect("virtual mode should not fail while pending work is not ready");
    assert!(output.is_none(), "virtual mode should report no output");
}

/// Advances virtual time to the next deadline before dispatching the timer task.
#[test]
fn test_runtime_tick_advances_virtual_time_before_dispatch() {
    // configure one virtual runtime with one future timer
    let options = runtime_options_with_execution(ExecutionMode::Strict);
    let mut runtime = TestWorldRuntime::build(&options, TestEngine::default());
    let default_worker_id = runtime.default_worker_id();
    let mono_before = runtime.mono_nanos();
    let fire_at_nanos = runtime.wall_nanos().saturating_add(5_000);
    let continuation = runtime.completing_continuation(default_worker_id, 111);
    runtime.with_worker_mut(default_worker_id, |worker| {
        register_timer_waiter(worker, 950, continuation, 0);
        schedule_timer(worker, TimerClock::Wall, 950, fire_at_nanos, None);
    });

    // the first tick should only advance time
    let outcome = runtime.tick();
    assert_eq!(outcome, TickResult::TimeAdvanced);
    assert_eq!(
        runtime.wall_nanos(),
        fire_at_nanos,
        "virtual time should jump exactly to the next deadline"
    );
    assert_eq!(
        runtime.mono_nanos(),
        mono_before.saturating_add(5_000),
        "virtual monotonic time should advance by the same delta"
    );

    // the next tick should dispatch the newly ready timer task
    let outcome = runtime.tick();
    assert_eq!(outcome, TickResult::Progress);
}

/// Executes one world tick through the attached runtime.
#[test]
fn test_world_tick_drives_runtime() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let mut world = World::new(&options, Environment::default()).expect("world");
    let runtime_id = world
        .spawn_runtime(
            destack_workspace::Environment::default(),
            &options,
            TestEngine::default(),
        )
        .expect("runtime should spawn");
    // enqueue one ready task on the default worker
    let World {
        state,
        runtimes,
        host,
        host_queue,
        ..
    } = &mut world;
    let runtime = runtimes.get_mut(&runtime_id).expect("runtime should exist");
    let default_worker_id = runtime.default_worker_id();
    runtime
        .with_worker_context(default_worker_id, |shared, runtime_static, worker| {
            let continuation = start_worker_continuation(
                worker,
                host.as_ref(),
                host_queue,
                state,
                shared,
                runtime_static,
                "test.complete",
                211,
            );
            worker.event_loop.enqueue_task(Task {
                id: TaskId::new(1),
                runnable: continuation,
                resume_value: engine::Value::Void,
                priority: 0,
            });
        })
        .expect("default worker should exist");

    // world tick should delegate through the runtime and execute the task
    assert_eq!(world.tick().expect("world tick"), TickResult::Progress);
}

/// Dispatches equal-deadline timers in stable worker-id order.
#[test]
fn test_runtime_tick_orders_equal_deadline_timers_by_worker_id() {
    // configure one virtual runtime with two workers and one equal deadline
    let options = runtime_options_with_execution(ExecutionMode::Strict);
    let mut runtime = TestWorldRuntime::build(&options, TestEngine::default());
    let default_worker_id = runtime.default_worker_id();
    let secondary_worker_id = runtime.spawn_worker(TestEngine::default());
    let fire_at_nanos = runtime.wall_nanos().saturating_add(10_000);
    let default_continuation = runtime.completing_continuation(default_worker_id, 201);
    let secondary_continuation = runtime.completing_continuation(secondary_worker_id, 202);

    // register one waiter timer on each worker
    runtime.with_worker_mut(default_worker_id, |worker| {
        register_timer_waiter(worker, 960, default_continuation, 0);
        schedule_timer(worker, TimerClock::Wall, 960, fire_at_nanos, None);
    });
    runtime.with_worker_mut(secondary_worker_id, |worker| {
        register_timer_waiter(worker, 961, secondary_continuation, 0);
        schedule_timer(worker, TimerClock::Wall, 961, fire_at_nanos, None);
    });

    // the first tick advances time and later ticks dispatch in worker order
    assert_eq!(runtime.tick(), TickResult::TimeAdvanced);
    assert_eq!(runtime.tick(), TickResult::Progress);
    assert_eq!(runtime.tick(), TickResult::Progress);
    assert_eq!(runtime.tick(), TickResult::Idle);
}

/// Advances virtual time to one simulation deadline when no worker work is ready.
#[test]
fn test_runtime_tick_advances_to_simulation_deadline() {
    // configure one virtual runtime with one simulated wakeup
    let options = runtime_options_with_execution(ExecutionMode::Strict);
    let runtime = TestWorldRuntime::build(&options, TestEngine::default());
    let mut runtime = runtime;
    let wall_before = runtime.wall_nanos();
    let mono_before = runtime.mono_nanos();
    let deadline = Instant::new(wall_before.saturating_add(7_500));
    runtime
        .world_mut()
        .simulation_mut()
        .schedule_event(deadline)
        .expect("schedule simulation event");

    // the first tick should advance world time to the simulated deadline
    let outcome = runtime.tick();
    assert_eq!(outcome, TickResult::TimeAdvanced);
    assert_eq!(runtime.wall_nanos(), deadline.get());
    assert_eq!(runtime.mono_nanos(), mono_before.saturating_add(7_500));
    let world = runtime.world();
    let simulation = world.simulation();
    assert_eq!(simulation.ready_events().len(), 1);
    assert_eq!(simulation.ready_events()[0].at(), deadline);
}

/// Register one timer waiter on one explicit worker.
fn register_timer_waiter(
    worker: &mut Worker,
    handle: u64,
    continuation: Continuation,
    priority: u8,
) {
    worker
        .add_timer_waiter(
            ResourceId::new(worker.worker_id(), handle),
            continuation,
            engine::Value::Void,
            priority,
        )
        .expect("timer waiter should register");
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
        .schedule_timer(ScheduledTimer {
            resource_id: ResourceId::new(worker.worker_id(), handle),
            deadline: TimerDeadline {
                clock,
                at: Nanos::new(fire_at_nanos),
            },
            interval: interval_nanos.map(Nanos::new),
        })
        .expect("timer should schedule");
}

/// Build one native continuation for mismatch tests.
fn native_continuation(value: u64) -> Continuation {
    Continuation::Native(native::Continuation::new(engine::Continuation {
        engine_id: engine::EngineId::new(value),
        frames: Vec::new(),
    }))
}
