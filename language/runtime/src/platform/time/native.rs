#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::time::bindings_generated as bindings;

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::time::{ClockId, ClockInfo, SleepClock};

/// Stub for destack.time.clock.info.
pub unsafe fn destack_time_clock_info(
    context: &RuntimeCallContext,
    out: *mut ClockInfo,
    clock: ClockId,
) -> RuntimeResult<()> {
    context.check_policy(TIME_CLOCK_INFO)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, clock);

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.clock.info")).boxed())
}

/// Stub for destack.time.clock.monoNs.
pub unsafe fn destack_time_mono_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context.check_policy(TIME_CLOCK_MONO_NS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.clock.monoNs")).boxed())
}

/// Stub for destack.time.clock.nowNs.
pub unsafe fn destack_time_now_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
    clock: ClockId,
) -> RuntimeResult<()> {
    context.check_policy(TIME_CLOCK_NOW_NS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, clock);

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.clock.nowNs")).boxed())
}

/// Stub for destack.time.clock.processCpuNs.
pub unsafe fn destack_time_process_cpu_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context.check_policy(TIME_CLOCK_PROCESS_CPU_NS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.processCpuNs",
    ))
    .boxed())
}

/// Stub for destack.time.clock.threadCpuNs.
pub unsafe fn destack_time_thread_cpu_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context.check_policy(TIME_CLOCK_THREAD_CPU_NS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.threadCpuNs",
    ))
    .boxed())
}

/// Stub for destack.time.clock.wallNs.
pub unsafe fn destack_time_wall_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context.check_policy(TIME_CLOCK_WALL_NS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.clock.wallNs")).boxed())
}

/// Stub for destack.time.sleep.ns.
pub unsafe fn destack_time_sleep_ns(
    context: &RuntimeCallContext,
    duration: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIME_SLEEP_NS)?;
    let _ = duration;

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.sleep.ns")).boxed())
}

/// Stub for destack.time.sleep.onNs.
pub unsafe fn destack_time_sleep_on_ns(
    context: &RuntimeCallContext,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    context.check_policy(TIME_SLEEP_ON_NS)?;
    let _ = (duration, clock);

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.sleep.onNs")).boxed())
}

/// Stub for destack.time.sleep.untilNs.
pub unsafe fn destack_time_sleep_until_ns(
    context: &RuntimeCallContext,
    deadline: u64,
) -> RuntimeResult<()> {
    context.check_policy(TIME_SLEEP_UNTIL_NS)?;
    let _ = deadline;

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.sleep.untilNs")).boxed())
}

/// Stub for destack.time.sleep.untilOnNs.
pub unsafe fn destack_time_sleep_until_on_ns(
    context: &RuntimeCallContext,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    context.check_policy(TIME_SLEEP_UNTIL_ON_NS)?;
    let _ = (deadline, clock);

    Err(RuntimeError::from(PlatformError::not_supported("destack.time.sleep.untilOnNs")).boxed())
}
