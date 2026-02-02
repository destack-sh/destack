use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;

/// Return wall clock time in nanoseconds.
pub fn destack_time_wall_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Ok(runtime.runtime().time.wall_nanos())
}

/// Return monotonic time in nanoseconds.
pub fn destack_time_mono_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Ok(runtime.runtime().time.mono_nanos())
}

/// Sleep for the given duration in nanoseconds.
pub fn destack_time_sleep_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    duration: u64,
) -> RuntimeResult<()> {
    runtime.runtime().time.sleep_nanos(duration);
    Ok(())
}
