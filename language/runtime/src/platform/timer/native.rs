use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{TimerFdHandle, TimerHandle};
use crate::platform::timer::{
    TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpec, TimerOptions,
    bindings_generated as bindings,
};
use crate::platform::{PlatformError, ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;
use crate::scheduler::Timer;

use bindings::*;

/// Return a not supported error for a timer binding.
fn not_supported(binding_name: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed())
}

/// Schedule a timer and write the handle.
unsafe fn schedule_timer(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    fire_at_nanos: u64,
    interval_nanos: Option<u64>,
) -> RuntimeResult<()> {
    // validate pointers before writing the result
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate a timer resource id
    let resource_id = context
        .runtime()
        .resources
        .insert(ResourceEntry::new(ResourceKind::Timer));
    let handle = TimerHandle(resource_id);

    // schedule the timer in the runtime event loop
    let timer = Timer {
        handle: resource_id,
        fire_at_nanos,
        interval_nanos,
    };
    context.scheduler().schedule_timer(timer)?;

    // write the handle to the output location
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Schedule a one shot timer for a relative delay.
pub unsafe fn destack_timer_once(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    delayns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_ONCE)?;

    // NOTE #Incomplete: honor timer options
    let _ = options;

    // compute the absolute deadline from wall time
    let now = context.runtime().time.wall_nanos();
    let fire_at_nanos = now.saturating_add(delayns);

    unsafe { schedule_timer(context, out, fire_at_nanos, None) }
}

/// Schedule a repeating timer for a fixed period.
pub unsafe fn destack_timer_interval(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    periodns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_INTERVAL)?;

    // NOTE #Incomplete: honor timer options
    let _ = options;

    // compute the first deadline from wall time
    let now = context.runtime().time.wall_nanos();
    let fire_at_nanos = now.saturating_add(periodns);

    unsafe { schedule_timer(context, out, fire_at_nanos, Some(periodns)) }
}

/// Cancel a scheduled timer.
pub unsafe fn destack_timer_cancel(
    context: &RuntimeCallContext,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_CANCEL)?;

    context.scheduler().cancel_timer(handle.0)
}

/// Schedule a timer for an absolute wall-clock deadline.
pub unsafe fn destack_timer_at(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    deadlinens: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_AT)?;

    // NOTE #Incomplete: honor timer options
    let _ = options;

    // schedule the absolute deadline as given
    unsafe { schedule_timer(context, out, deadlinens, None) }
}

/// Schedule a timer for an absolute deadline with options.
pub unsafe fn destack_timer_at_with_options(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    deadlinens: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_AT)?;
    unsafe { destack_timer_at(context, out, deadlinens, options) }
}

/// Schedule a one shot timer with options.
pub unsafe fn destack_timer_once_with_options(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    delayns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_ONCE)?;

    // NOTE #Incomplete: honor timer options
    let _ = options;

    unsafe { destack_timer_once(context, out, delayns, options) }
}

/// Schedule an interval timer with options.
pub unsafe fn destack_timer_interval_with_options(
    context: &RuntimeCallContext,
    out: *mut TimerHandle,
    periodns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_INTERVAL)?;

    // NOTE #Incomplete: honor timer options
    let _ = options;

    unsafe { destack_timer_interval(context, out, periodns, options) }
}

/// Report whether a timer is active.
pub unsafe fn destack_timer_is_active(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_IS_ACTIVE)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    // NOTE #Incomplete: implement timer activity query
    not_supported("destack.timer.isActive")
}

/// Pause a timer.
pub unsafe fn destack_timer_pause(
    context: &RuntimeCallContext,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_PAUSE)?;

    // NOTE #Incomplete: implement timer pause
    let _ = handle;
    not_supported("destack.timer.pause")
}

/// Resume a paused timer.
pub unsafe fn destack_timer_resume(
    context: &RuntimeCallContext,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_RESUME)?;

    // NOTE #Incomplete: implement timer resume
    let _ = handle;
    not_supported("destack.timer.resume")
}

/// Read remaining nanoseconds for a timer.
pub unsafe fn destack_timer_remaining_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_REMAINING_NS)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    // NOTE #Incomplete: implement remaining-time query
    not_supported("destack.timer.remainingNs")
}

/// Reset a timer delay.
pub unsafe fn destack_timer_reset(
    context: &RuntimeCallContext,
    handle: TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_RESET)?;

    // NOTE #Incomplete: implement timer reset
    let _ = (handle, delayns);
    not_supported("destack.timer.reset")
}

/// Update an interval timer period.
pub unsafe fn destack_timer_update_interval(
    context: &RuntimeCallContext,
    handle: TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_UPDATE_INTERVAL)?;

    // NOTE #Incomplete: implement interval update
    let _ = (handle, periodns);
    not_supported("destack.timer.updateInterval")
}

/// Close a timerfd style descriptor.
pub unsafe fn destack_timer_timer_fd_close(
    context: &RuntimeCallContext,
    handle: TimerFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_FD_CLOSE)?;

    // NOTE #Incomplete: implement timerfd close in native mode
    let _ = handle;
    not_supported("destack.timer.timerFdClose")
}

/// Read a timerfd specification.
pub unsafe fn destack_timer_timer_fd_get(
    context: &RuntimeCallContext,
    out: *mut TimerFdSpec,
    handle: TimerFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_FD_GET)?;

    // validate output pointers before writing
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // NOTE #Incomplete: implement timerfd get in native mode
    let _ = (out, handle);
    not_supported("destack.timer.timerFdGet")
}

/// Open a timerfd style descriptor.
pub unsafe fn destack_timer_timer_fd_open(
    context: &RuntimeCallContext,
    out: *mut TimerFdHandle,
    clock: TimerFdClock,
    flags: TimerFdFlags,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_FD_OPEN)?;

    // validate output pointers before writing
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // NOTE #Incomplete: implement timerfd open in native mode
    let _ = (out, clock, flags);
    not_supported("destack.timer.timerFdOpen")
}

/// Read timer expirations from a timerfd descriptor.
pub unsafe fn destack_timer_timer_fd_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: TimerFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_FD_READ)?;

    // validate output pointers before writing
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // NOTE #Incomplete: implement timerfd read in native mode
    let _ = (out, handle);
    not_supported("destack.timer.timerFdRead")
}

/// Update a timerfd specification.
pub unsafe fn destack_timer_timer_fd_set(
    context: &RuntimeCallContext,
    handle: TimerFdHandle,
    spec: TimerFdSpec,
    flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_FD_SET)?;

    // NOTE #Incomplete: implement timerfd set in native mode
    let _ = (handle, spec, flags);
    not_supported("destack.timer.timerFdSet")
}
