use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::replay::{ReplayEvent, TimeEvent, TimeEventKind};
use crate::runtime::RuntimeCallContext;

/// Return wall clock time in nanoseconds.
pub fn destack_time_wall_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    let value = runtime.runtime().time.wall_nanos();
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::WallClockRead,
            time_nanos: value,
            interval_nanos: None,
            timer_id: None,
        }));
    Ok(value)
}

/// Return monotonic time in nanoseconds.
pub fn destack_time_mono_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    let value = runtime.runtime().time.mono_nanos();
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::MonotonicSample,
            time_nanos: value,
            interval_nanos: None,
            timer_id: None,
        }));
    Ok(value)
}

/// Sleep for the given duration in nanoseconds.
pub fn destack_time_sleep_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    duration: u64,
) -> RuntimeResult<()> {
    let start = runtime.runtime().time.wall_nanos();
    let deadline = start.saturating_add(duration);
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::SleepScheduled,
            time_nanos: deadline,
            interval_nanos: None,
            timer_id: None,
        }));
    runtime.runtime().time.sleep_nanos(duration);
    let wake = runtime.runtime().time.wall_nanos();
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::SleepWake,
            time_nanos: wake,
            interval_nanos: None,
            timer_id: None,
        }));
    Ok(())
}

/// Sleep until the provided deadline in nanoseconds.
pub fn destack_time_sleep_until_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    deadline: u64,
) -> RuntimeResult<()> {
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::SleepScheduled,
            time_nanos: deadline,
            interval_nanos: None,
            timer_id: None,
        }));
    runtime.runtime().time.sleep_until_nanos(deadline);
    let wake = runtime.runtime().time.wall_nanos();
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::SleepWake,
            time_nanos: wake,
            interval_nanos: None,
            timer_id: None,
        }));
    Ok(())
}
