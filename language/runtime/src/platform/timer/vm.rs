use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::TimerHandle;
use crate::platform::{ResourceEntry, ResourceKind};
use crate::replay::{ReplayEvent, TimeEvent, TimeEventKind};
use crate::runtime::RuntimeCallContext;
use crate::scheduler::Timer;

/// Schedule a oneshot timer and return its handle.
pub fn destack_timer_once(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    delayns: u64,
) -> RuntimeResult<TimerHandle> {
    let handle = schedule_timer(runtime, delayns, None)?;
    Ok(handle)
}

/// Schedule a repeating timer and return its handle.
pub fn destack_timer_interval(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    periodns: u64,
) -> RuntimeResult<TimerHandle> {
    let handle = schedule_timer(runtime, periodns, Some(periodns))?;
    Ok(handle)
}

/// Cancel a scheduled timer.
pub fn destack_timer_cancel(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    cancel_timer(runtime, handle)?;
    Ok(())
}

/// Schedule a timer and return its handle.
fn schedule_timer(
    runtime: &RuntimeCallContext,
    delay_nanos: u64,
    interval_nanos: Option<u64>,
) -> RuntimeResult<TimerHandle> {
    let resource_id = runtime
        .runtime()
        .resources
        .insert(ResourceEntry::new(ResourceKind::Timer));
    let handle = TimerHandle(resource_id);
    let fire_at = runtime
        .runtime()
        .time
        .wall_nanos()
        .saturating_add(delay_nanos);
    let timer = Timer {
        handle: resource_id,
        fire_at_nanos: fire_at,
        interval_nanos,
    };

    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::TimerScheduled,
            time_nanos: fire_at,
            interval_nanos,
            timer_id: Some(resource_id.0),
        }));
    runtime.scheduler().schedule_timer(timer)?;

    Ok(handle)
}

/// Cancel a timer by handle.
fn cancel_timer(runtime: &RuntimeCallContext, handle: TimerHandle) -> RuntimeResult<()> {
    runtime.scheduler().cancel_timer(handle.0)?;
    let resource_id = handle.0;
    let timer_id = resource_id.0;
    runtime
        .runtime()
        .replay
        .record_event(ReplayEvent::TimeEvent(TimeEvent {
            kind: TimeEventKind::TimerCanceled,
            time_nanos: runtime.runtime().time.wall_nanos(),
            interval_nanos: None,
            timer_id: Some(timer_id),
        }));
    Ok(())
}
