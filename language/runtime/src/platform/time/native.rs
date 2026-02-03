use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;

/// Return wall clock time in nanoseconds for native code.
pub unsafe fn destack_time_wall_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let value = context.runtime().time.wall_nanos();
    unsafe {
        *out = value;
    }
    Ok(())
}

/// Return monotonic time in nanoseconds for native code.
pub unsafe fn destack_time_mono_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let value = context.runtime().time.mono_nanos();
    unsafe {
        *out = value;
    }
    Ok(())
}

/// Sleep for the given duration for native code.
pub unsafe fn destack_time_sleep_ns(
    context: &RuntimeCallContext,
    duration_nanos: u64,
) -> RuntimeResult<()> {
    context.runtime().time.sleep_nanos(duration_nanos);
    Ok(())
}

/// Sleep until the given deadline for native code.
pub unsafe fn destack_time_sleep_until_ns(
    context: &RuntimeCallContext,
    deadline_nanos: u64,
) -> RuntimeResult<()> {
    context.runtime().time.sleep_until_nanos(deadline_nanos);
    Ok(())
}
