use std::sync::Arc;

use tspp_memory::MemoryMap;
use tspp_program as program;

use crate::host::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::host::time::TimerClock;
use crate::host::{HostEvent, LifecycleEvent, LifecycleSourceKind, LifecycleState, ResourceId};
use crate::scheduler::{
    Callback, EventLoop, HostWake, Invocation, Readiness, ResourceWake, ScheduledTimer,
    TimerDeadline, Wake,
};
use crate::worker::WorkerId;
use crate::world::time::Nanos;

/// Dequeues microtasks before tasks.
#[test]
fn test_event_loop_drains_microtasks_before_tasks() {
    // set up an event loop with one task and one microtask
    let mut event_loop = EventLoop::default();
    let task_id = event_loop.enqueue_task(Invocation::call(
        program::FunctionId(0),
        [],
        program::Context::empty(),
    ));
    let microtask_id = event_loop.enqueue_microtask(Invocation::call(
        program::FunctionId(0),
        [],
        program::Context::empty(),
    ));

    // verify microtask dispatch precedes task dispatch
    let first = event_loop
        .pop_microtask()
        .expect("microtask should dequeue first");
    assert_eq!(first.id, microtask_id);

    let second = event_loop.pop_task().expect("task should dequeue second");
    assert_eq!(second.id, task_id);
}

/// Forks queued scheduler state without dropping pending work.
#[test]
fn test_event_loop_fork_preserves_pending_state() {
    // one queued resource wake and one ready timer
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_wake(resource_wake(61, 991));
    event_loop
        .schedule_timer(ScheduledTimer {
            resource_id: resource_id(62),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("schedule timer");

    // fork the active scheduler directly over fresh world memory
    let memory = Arc::new(MemoryMap::reserve(1 << 20, 1 << 16).expect("reserve fork memory"));
    let mut forked = event_loop
        .fork(&memory)
        .expect("active event loop should fork");

    // ready timer stays ahead of queued resource wakes
    let first = forked
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("dequeue first wake");
    assert!(matches!(first, Some(Wake::Timer(_))));

    let second = forked
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("dequeue second wake");
    assert!(matches!(
        second,
        Some(Wake::Resource(wake)) if wake.readiness == Readiness::Readable
    ));

    // forked allocation continues from the same deterministic identity
    let expected_id = event_loop.enqueue_task(Invocation::call(
        program::FunctionId(0),
        [],
        program::Context::empty(),
    ));
    let forked_id = forked.enqueue_task(Invocation::call(
        program::FunctionId(0),
        [],
        program::Context::empty(),
    ));
    assert_eq!(forked_id, expected_id);
}

/// Drops ready timers that were canceled before dispatch.
#[test]
fn test_event_loop_cancel_timer_drops_ready_timer_before_dispatch() {
    // enqueue one timer and cancel it before dispatch
    let mut event_loop = EventLoop::default();
    event_loop
        .schedule_timer(ScheduledTimer {
            resource_id: resource_id(700),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("timer should schedule");

    // cancel before dequeue and verify dispatch is suppressed
    event_loop
        .cancel_timer(resource_id(700))
        .expect("timer cancel should succeed");
    let next = event_loop
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("event loop should dequeue wake");
    assert!(
        next.is_none(),
        "canceled ready timer should not dispatch as runnable work"
    );
}

/// Identifies each dispatch of one reusable waiter independently.
#[test]
fn test_event_loop_identifies_repeated_dispatches() {
    let resource_id = resource_id(701);
    let mut event_loop = EventLoop::default();
    event_loop.add_resource_waiter(
        resource_id,
        Readiness::Readable,
        Callback::call(program::FunctionId(0), [], program::Context::empty()),
    );
    let wake = Wake::Resource(ResourceWake::poller(PollerEvent {
        resource_id,
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(0),
        payload: PollerEventPayload::Io { data: 0 },
    }));

    // dispatch the reusable invocation twice
    let first_id = event_loop
        .dispatch(wake.clone())
        .expect("registered waiter should dispatch");
    let second_id = event_loop
        .dispatch(wake)
        .expect("registered waiter should dispatch again");

    // each queued execution receives its own identity
    let first = event_loop.pop_task().expect("first task should be queued");
    let second = event_loop.pop_task().expect("second task should be queued");
    assert_eq!(first.id, first_id);
    assert_eq!(second.id, second_id);
    assert_ne!(first.id, second.id);
}

/// Dequeues external wakes in deterministic ingress order.
#[test]
fn test_event_loop_dequeues_wakes_in_ingress_order() {
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_wake(resource_wake(702, 8));
    let event = HostEvent::Lifecycle(LifecycleEvent {
        source_kind: LifecycleSourceKind::Application,
        state: LifecycleState::Running,
    });
    event_loop.enqueue_wake(Wake::Host(HostWake::new(event.clone())));

    let first = event_loop
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("first wake should dequeue");
    let second = event_loop
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("second wake should dequeue");

    assert!(matches!(
        first,
        Some(Wake::Resource(wake)) if wake.readiness == Readiness::Readable
    ));
    assert!(matches!(second, Some(Wake::Host(wake)) if wake.event == event));
}

/// Dequeues due timers before external ingress wakes.
#[test]
fn test_event_loop_prioritizes_due_timers() {
    let mut event_loop = EventLoop::default();
    event_loop.enqueue_wake(resource_wake(703, 9));
    event_loop
        .schedule_timer(ScheduledTimer {
            resource_id: resource_id(704),
            deadline: TimerDeadline {
                clock: TimerClock::Wall,
                at: Nanos::new(0),
            },
            interval: None,
        })
        .expect("timer should schedule");

    let first = event_loop
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("timer wake should dequeue");
    let second = event_loop
        .next_wake(Nanos::new(0), Nanos::new(0))
        .expect("resource wake should dequeue");

    assert!(matches!(first, Some(Wake::Timer(_))));
    assert!(matches!(
        second,
        Some(Wake::Resource(wake)) if wake.readiness == Readiness::Readable
    ));
}

/// Build one readable scheduler resource wake.
fn resource_wake(local_id: u64, token: u64) -> Wake {
    Wake::Resource(ResourceWake::poller(PollerEvent {
        resource_id: resource_id(local_id),
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(token),
        payload: PollerEventPayload::Io { data: 0 },
    }))
}

/// Build one scheduler test resource id.
fn resource_id(local_id: u64) -> ResourceId {
    ResourceId::new(WorkerId(1), local_id)
}
