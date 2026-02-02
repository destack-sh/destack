use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::TimerHandle;
use crate::platform::{ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;
use crate::scheduler::Timer;

/// Schedule a one-shot timer and return its handle.
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

    runtime.scheduler().schedule_timer(timer)?;

    Ok(handle)
}

fn cancel_timer(runtime: &RuntimeCallContext, handle: TimerHandle) -> RuntimeResult<()> {
    runtime.scheduler().cancel_timer(handle.0)?;
    Ok(())
}
