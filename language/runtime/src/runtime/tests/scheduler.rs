use std::sync::Arc;

use destack_core::{Capture, CaptureMode};
use destack_workspace::{RuntimeOptions, SchedulerOptions, TimeMode};
use {destack_engine as engine, destack_native as native};

use crate::host::{HostEventKind, HostLifecycleState, Session};
use crate::platform::ResourceId;
use crate::platform::time::TimerClock;
use crate::runtime::engine::{Continuation, Engine};
use crate::runtime::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::runtime::scheduler::{
    EventLoop, Microtask, MicrotaskId, Runnable, Task, TaskId, TaskStatus, Timer, TimerDeadline,
};
use crate::runtime::time::{Instant, Nanos, host as host_time};
use crate::runtime::{DropReason, TickResult, Worker, WorkerOptions, World};

use super::tests::{
    TestEngine, TestHostClockSource, TestMultiAgentRuntime, TestPoller, TestRuntime,
};

/// Build runtime options with one explicit time mode.
fn runtime_options_with_time_mode(mode: TimeMode) -> RuntimeOptions {
    let mut options = RuntimeOptions::default();
    options.set_time_mode(mode);

    options
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
    assert!(
        runtime.has_pending_work(),
        "yielded task should stay queued"
    );
}

/// Drains queued tasks and re-yields until idle.
#[test]
fn test_tick_until_idle_drains_yielded_tasks() {
    // create runtime state with one queued task
    let mut runtime = TestRuntime::new();
    runtime.enqueue_task_native(11, 9, 0);

    // run ticks until the queue is drained
    runtime.tick_until_idle();

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
    assert!(
        runtime.has_pending_work(),
        "yielded timer task should stay queued"
    );

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
    assert!(
        runtime.has_pending_work(),
        "yielded event task should stay queued"
    );
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
    assert!(
        runtime.has_pending_work(),
        "yielded host event task should stay queued"
    );
}

/// Routes event watches into the task queue and preserves priority ordering.
#[test]
fn test_tick_routes_event_watch_through_task_priority() {
    // create runtime state with one queued high-priority task
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), TestEngine::default());
    runtime.enqueue_task_native(301, 91, 200);

    // register one low-priority event watch and enqueue one event
    runtime.watch_event_native(44, 92, 0);
    runtime.enqueue_io_event(7, 44, 1);

    // run high-priority task before watched event work
    let _ = runtime.tick();
    assert!(
        runtime.has_pending_work(),
        "watched event should remain after the high-priority task"
    );

    let _ = runtime.tick();
    assert!(
        runtime.has_pending_work(),
        "yielded watched event task should stay queued"
    );
}

/// Drops queued events that have no registered dispatch watch.
#[test]
fn test_tick_drops_event_without_watch() {
    // create runtime state with one unregistered event token
    let mut runtime =
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), TestEngine::default());
    runtime.enqueue_io_event(8, 404, 2);

    // executing one tick should drop the stale event without crashing
    let progressed = runtime.tick();
    assert!(
        progressed,
        "dropping one queued event should count as progress"
    );
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
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), TestEngine::default());
    runtime.enqueue_lifecycle_host_event(HostLifecycleState::Running);

    // executing one tick should drop the stale event without crashing
    let progressed = runtime.tick();
    assert!(
        progressed,
        "dropping one queued host event should count as progress"
    );
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
        TestEngine::default(),
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

    assert_eq!(outcome, TickResult::Worked);
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
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), TestEngine::default());
    runtime.configure_scheduler(SchedulerOptions {
        microtask_budget: Some(1),
        ..SchedulerOptions::default()
    });

    // enqueue two microtasks and run one tick
    runtime.enqueue_microtask_native(1, 51);
    runtime.enqueue_microtask_native(2, 52);

    let _ = runtime.tick();

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
        TestRuntime::with_options_and_engine(&RuntimeOptions::default(), TestEngine::default());
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
    assert!(!runtime.has_microtasks(), "both microtasks should complete");
}

/// Dequeues microtasks before macrotasks.
#[test]
fn test_event_loop_next_runnable_prioritizes_microtasks() {
    // set up an event loop with one task and one microtask
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Task {
        id: TaskId::new(501),
        runnable: native_continuation(601),
        resume_value: engine::Value::Void,
        status: TaskStatus::Ready,
        priority: 0,
    });
    event_loop.enqueue_microtask(Microtask {
        id: MicrotaskId::new(502),
        continuation: native_continuation(602),
        resume_value: engine::Value::Void,
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
        runnable: native_continuation(603),
        resume_value: engine::Value::Void,
        status: TaskStatus::Ready,
        priority: 1,
    });
    event_loop.enqueue_task(Task {
        id: TaskId::new(504),
        runnable: native_continuation(604),
        resume_value: engine::Value::Void,
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
        runnable: native_continuation(701),
        resume_value: engine::Value::Void,
        status: TaskStatus::Ready,
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
    let mut engine = Engine::from(TestEngine::default());
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
    let options = runtime_options_with_time_mode(TimeMode::Virtual);
    let mut runtime = TestRuntime::with_options_and_engine(&options, TestEngine::default());

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
    // configure one test host clock source for deterministic host mode waits
    let host_clock_source = Arc::new(TestHostClockSource::new(1_000_000, 0));
    let options = runtime_options_with_time_mode(TimeMode::Host);
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        TestEngine::default(),
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
        "host wait should advance test monotonic time"
    );
}

/// Waits in host mode until one future timer dispatches the target task.
#[test]
fn test_run_loop_until_task_complete_waits_for_host_timer() {
    // configure one test host clock source for deterministic host mode waits
    let host_clock_source = Arc::new(TestHostClockSource::new(2_000_000, 0));
    let options = runtime_options_with_time_mode(TimeMode::Host);
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        TestEngine::default(),
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
    assert_eq!(output, engine::Value::int32(88));
    assert!(
        host_clock_source.wall_nanos() >= fire_at_nanos,
        "host wait should advance test wall time to the timer deadline"
    );
}

/// Dispatches wall timers after one wall-clock jump without monotonic advancement.
#[test]
fn test_wall_clock_jump_fires_wall_timer() {
    // configure one host-mode runtime with one test clock source
    let host_clock_source = Arc::new(TestHostClockSource::new(10_000, 500));
    let options = runtime_options_with_time_mode(TimeMode::Host);
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        TestEngine::default(),
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
    assert_eq!(
        runtime.mono_nanos(),
        500,
        "wall jump should not implicitly advance monotonic time"
    );
}

/// Keeps monotonic timers pending across wall-clock jumps.
#[test]
fn test_wall_clock_jump_does_not_fire_monotonic_timer() {
    // configure one host-mode runtime with one test clock source
    let host_clock_source = Arc::new(TestHostClockSource::new(20_000, 900));
    let options = runtime_options_with_time_mode(TimeMode::Host);
    let mut runtime = TestRuntime::with_options_engine_and_host_clock_source(
        &options,
        TestEngine::default(),
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

    // monotonic advancement should make the timer ready
    host_clock_source.advance_mono_nanos(1_000);
    let progressed = runtime.tick();
    assert!(
        progressed,
        "monotonic timer should dispatch once monotonic time advances"
    );
}

/// Advances virtual time to the next deadline before dispatching the timer task.
#[test]
fn test_runtime_tick_advances_virtual_time_before_dispatch() {
    // configure one virtual runtime with one future timer
    let options = runtime_options_with_time_mode(TimeMode::Virtual);
    let mut runtime =
        TestMultiAgentRuntime::with_options_and_engine(&options, TestEngine::default());
    let default_worker_id = runtime.default_worker_id();
    let fire_at_nanos = runtime.wall_nanos().saturating_add(5_000);
    let continuation = runtime.completing_continuation(default_worker_id, 111);
    runtime.with_worker_mut(default_worker_id, |worker| {
        register_timer_watch(worker, 950, continuation, 0);
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
        fire_at_nanos,
        "virtual monotonic time should advance with wall time"
    );

    // the next tick should dispatch the newly ready timer task
    let outcome = runtime.tick();
    assert_eq!(outcome, TickResult::Worked);
}

/// Executes one world tick through the attached runtime.
#[test]
fn test_world_tick_drives_runtime() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("world");
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, TestEngine::default())
        .expect("runtime should spawn");
    // enqueue one ready task on the default worker
    let runtime = world.runtime_mut(runtime_id).expect("runtime should exist");
    let default_worker_id = runtime.default_worker_id();
    runtime
        .with_worker_context(default_worker_id, |shared, runtime_static, worker| {
            let continuation = super::tests::start_worker_continuation(
                worker,
                shared,
                runtime_static,
                "test.complete",
                211,
            );
            worker.event_loop.enqueue_task(Task {
                id: TaskId::new(1),
                runnable: continuation,
                resume_value: engine::Value::Void,
                status: TaskStatus::Ready,
                priority: 0,
            });
        })
        .expect("default worker should exist");

    // world tick should delegate through the runtime and execute the task
    assert_eq!(world.tick().expect("world tick"), TickResult::Worked);
}

/// Dispatches equal-deadline timers in stable worker-id order.
#[test]
fn test_runtime_tick_orders_equal_deadline_timers_by_worker_id() {
    // configure one virtual runtime with two workers and one equal deadline
    let options = runtime_options_with_time_mode(TimeMode::Virtual);
    let mut runtime =
        TestMultiAgentRuntime::with_options_and_engine(&options, TestEngine::default());
    let default_worker_id = runtime.default_worker_id();
    let secondary_worker_id = runtime.spawn_worker(TestEngine::default());
    let fire_at_nanos = runtime.wall_nanos().saturating_add(10_000);
    let default_continuation = runtime.completing_continuation(default_worker_id, 201);
    let secondary_continuation = runtime.completing_continuation(secondary_worker_id, 202);

    // register one watched timer on each worker
    runtime.with_worker_mut(default_worker_id, |worker| {
        register_timer_watch(worker, 960, default_continuation, 0);
        schedule_timer(worker, TimerClock::Wall, 960, fire_at_nanos, None);
    });
    runtime.with_worker_mut(secondary_worker_id, |worker| {
        register_timer_watch(worker, 961, secondary_continuation, 0);
        schedule_timer(worker, TimerClock::Wall, 961, fire_at_nanos, None);
    });

    // the first tick advances time and later ticks dispatch in worker order
    assert_eq!(runtime.tick(), TickResult::TimeAdvanced);
    assert_eq!(runtime.tick(), TickResult::Worked);
    assert_eq!(runtime.tick(), TickResult::Worked);
    assert_eq!(runtime.tick(), TickResult::Idle);
}

/// Advances virtual time to one simulation deadline when no worker work is ready.
#[test]
fn test_runtime_tick_advances_to_simulation_deadline() {
    // configure one virtual runtime with one simulated wakeup
    let options = runtime_options_with_time_mode(TimeMode::Virtual);
    let runtime = TestMultiAgentRuntime::with_options_and_engine(&options, TestEngine::default());
    let mut runtime = runtime;
    runtime
        .world_mut()
        .simulation_mut()
        .schedule_event(Instant::new(7_500));

    // the first tick should advance world time to the simulated deadline
    let outcome = runtime.tick();
    assert_eq!(outcome, TickResult::TimeAdvanced);
    assert_eq!(runtime.wall_nanos(), 7_500);
    assert_eq!(runtime.mono_nanos(), 7_500);
    let world = runtime.world();
    let simulation = world.simulation();
    assert_eq!(simulation.ready_events().len(), 1);
    assert_eq!(simulation.ready_events()[0].at(), Instant::new(7_500));
}

/// Rejects synchronous virtual sleeps so time only advances through runtime ticks.
#[test]
fn test_virtual_sleep_binding_fails_loudly() {
    // build one virtual-time binding context
    let options = runtime_options_with_time_mode(TimeMode::Virtual);
    let mut world = World::from_options(&options).expect("world");
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let world_state = &mut world.state;

    let mut worker = Worker::new_in_world(
        Vec::new(),
        &options,
        world_state,
        &shared,
        &engine::StaticSpace::empty(),
        WorkerOptions::default(),
        TestEngine::default(),
    )
    .expect("worker should build");
    let host = Session::from_runtime_options(&options, worker.runtime_id);
    let binding = super::tests::binding_call_context(&mut worker, &host, world_state);
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

/// Register one timer watch on one explicit worker.
fn register_timer_watch(
    worker: &mut Worker,
    handle: u64,
    continuation: Continuation,
    priority: u8,
) {
    worker
        .watch_timer(
            ResourceId(handle),
            continuation,
            engine::Value::Void,
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

/// Build one native continuation for mismatch tests.
fn native_continuation(value: u64) -> Continuation {
    Continuation::Native(native::Continuation::new(engine::Continuation {
        engine_id: engine::EngineId::new(value),
        frames: Vec::new(),
    }))
}
