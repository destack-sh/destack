use crate::diagnostic::RuntimeResult;
use crate::platform::resource::TimerHandle;
use crate::platform::{ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;
use crate::scheduler::Timer;

/// Schedule a oneshot timer for native code.
pub unsafe fn destack_timer_once(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    delay_nanos: u64,
) -> RuntimeResult<()> {
    // allocate the timer handle
    let resource_id = context
        .runtime()
        .resources
        .insert(ResourceEntry::new(ResourceKind::Timer));
    let handle = TimerHandle(resource_id);
    let fire_at = context
        .runtime()
        .time
        .wall_nanos()
        .saturating_add(delay_nanos);

    // schedule the timer
    let timer = Timer {
        handle: resource_id,
        fire_at_nanos: fire_at,
        interval_nanos: None,
    };
    context.scheduler().schedule_timer(timer)?;

    // write the timer handle to the output pointer
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Schedule a repeating timer for native code.
pub unsafe fn destack_timer_interval(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    period_nanos: u64,
) -> RuntimeResult<()> {
    // allocate the timer handle
    let resource_id = context
        .runtime()
        .resources
        .insert(ResourceEntry::new(ResourceKind::Timer));
    let handle = TimerHandle(resource_id);
    let fire_at = context
        .runtime()
        .time
        .wall_nanos()
        .saturating_add(period_nanos);

    // schedule the timer
    let timer = Timer {
        handle: resource_id,
        fire_at_nanos: fire_at,
        interval_nanos: Some(period_nanos),
    };
    context.scheduler().schedule_timer(timer)?;

    // write the timer handle to the output pointer
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Cancel a scheduled timer for native code.
pub unsafe fn destack_timer_cancel(
    context: &RuntimeCallContext,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    context.scheduler().cancel_timer(handle.0)?;
    Ok(())
}
