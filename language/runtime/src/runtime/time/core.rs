use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::time::ClockMetadata;
use crate::runtime::RuntimeCallContext;
use destack_workspace::TimeMode;

/// Return one invalid-pointer error.
pub(crate) fn null_pointer_error(field: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::null_pointer(field)).boxed()
}

/// Return one unsupported runtime host time operation error.
#[cfg(unix)]
pub(crate) fn unsupported_host_operation_error(operation: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Return true when the runtime clock is virtualized.
pub(crate) fn is_virtual_clock(context: &RuntimeCallContext) -> bool {
    context.runtime().time.mode() == TimeMode::Virtual
}

/// Return one runtime-backed wall clock sample.
pub(crate) fn runtime_wall_nanos(context: &RuntimeCallContext) -> u64 {
    context.runtime().time.wall_nanos()
}

/// Return one runtime-backed monotonic clock sample.
pub(crate) fn runtime_mono_nanos(context: &RuntimeCallContext) -> u64 {
    context.runtime().time.mono_nanos()
}

/// Sleep one runtime-backed duration.
pub(crate) fn runtime_sleep_nanos(context: &RuntimeCallContext, duration: u64) {
    context.runtime().time.sleep_nanos(duration);
}

/// Sleep until one runtime-backed wall deadline.
pub(crate) fn runtime_sleep_until_wall_nanos(context: &RuntimeCallContext, deadline: u64) {
    context.runtime().time.sleep_until_nanos(deadline);
}

/// Sleep until one runtime-backed monotonic deadline.
pub(crate) fn runtime_sleep_until_mono_nanos(context: &RuntimeCallContext, deadline: u64) {
    let now = runtime_mono_nanos(context);
    if deadline <= now {
        return;
    }

    let delta = deadline.saturating_sub(now);
    runtime_sleep_nanos(context, delta);
}

/// Write one u64 output value.
pub(crate) unsafe fn write_out_u64(out: *mut u64, value: u64) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    unsafe {
        *out = value;
    }

    Ok(())
}

/// Write one clock-info output value.
pub(crate) unsafe fn write_out_clock_info(
    out: *mut ClockMetadata,
    value: ClockMetadata,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    unsafe {
        *out = value;
    }

    Ok(())
}
