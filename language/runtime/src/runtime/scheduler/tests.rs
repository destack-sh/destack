use destack_core::{Capture, CaptureMode};
use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::host::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::host::time::TimerClock;
use crate::runtime::scheduler::{
    EventLoop, Readiness, Runnable, RunnableId, ScheduledTimer, TimerDeadline, Wake,
};
use crate::runtime::time::Nanos;
use crate::tests::harness::{TestMachine, TestRuntime, test_resource_id};

/// Dequeues microtasks before macrotasks.
#[test]
fn test_event_loop_drains_microtasks_before_tasks() {
    // set up an event loop with one task and one microtask
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), TestMachine::default());
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_task(Runnable {
        id: RunnableId::new(501),
        continuation: runtime.yielding_continuation(601),
        resume_value: program::Value::Void,
    });
    event_loop.enqueue_microtask(Runnable {
        id: RunnableId::new(502),
        continuation: runtime.yielding_continuation(602),
        resume_value: program::Value::Void,
    });

    // verify microtask dispatch precedes task dispatch
    let first = event_loop.pop_microtask();
    assert!(first.is_some(), "microtask should dequeue first");

    let second = event_loop.pop_task();
    assert!(second.is_some(), "task should dequeue after microtasks");
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
    let image = event_loop
        .capture_image(CaptureMode::Suspend, ())
        .expect("capture suspend image");
    let mut restored = EventLoop::default();
    restored
        .restore_image(&image, ())
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
