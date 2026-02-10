#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::timer::bindings_generated as bindings;

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::timer::TimerOptions;

/// Stub for destack.timer.control.cancel.
pub unsafe fn destack_timer_cancel(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_CANCEL)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.control.cancel")).boxed())
}

/// Stub for destack.timer.control.isActive.
pub unsafe fn destack_timer_is_active(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_IS_ACTIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.isActive",
    ))
    .boxed())
}

/// Stub for destack.timer.control.pause.
pub unsafe fn destack_timer_pause(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_PAUSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.control.pause")).boxed())
}

/// Stub for destack.timer.control.remainingNs.
pub unsafe fn destack_timer_remaining_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_REMAINING_NS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.remainingNs",
    ))
    .boxed())
}

/// Stub for destack.timer.control.reset.
pub unsafe fn destack_timer_reset(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_RESET)?;
    let _ = (handle, delayns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.control.reset")).boxed())
}

/// Stub for destack.timer.control.resume.
pub unsafe fn destack_timer_resume(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_RESUME)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.control.resume")).boxed())
}

/// Stub for destack.timer.control.updateInterval.
pub unsafe fn destack_timer_update_interval(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_CONTROL_UPDATE_INTERVAL)?;
    let _ = (handle, periodns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.updateInterval",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.at.
pub unsafe fn destack_timer_at(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    deadlinens: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_AT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, deadlinens);

    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.schedule.at")).boxed())
}

/// Stub for destack.timer.schedule.atWithOptions.
pub unsafe fn destack_timer_at_with_options(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    deadlinens: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_AT_WITH_OPTIONS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, deadlinens, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.atWithOptions",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.interval.
pub unsafe fn destack_timer_interval(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_INTERVAL)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, periodns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.interval",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.intervalWithOptions.
pub unsafe fn destack_timer_interval_with_options(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    periodns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_INTERVAL_WITH_OPTIONS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, periodns, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.intervalWithOptions",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.once.
pub unsafe fn destack_timer_once(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_ONCE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, delayns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.schedule.once")).boxed())
}

/// Stub for destack.timer.schedule.onceWithOptions.
pub unsafe fn destack_timer_once_with_options(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    delayns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    context.check_policy(TIMER_SCHEDULE_ONCE_WITH_OPTIONS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, delayns, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.onceWithOptions",
    ))
    .boxed())
}
