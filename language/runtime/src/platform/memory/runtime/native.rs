use crate::diagnostic::RuntimeResult;
use crate::platform::memory::native as memory_native;
use crate::runtime::RuntimeCallContext;

/// Set runtime W^X policy.
///
/// Enable or disable runtime write-xor-execute policy enforcement.
/// Policy update affects subsequent executable-memory transitions.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime memory policy controls layered over host page protections.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `memory.execute`.
///
/// # Replay
/// Deterministic.
pub(crate) unsafe fn destack_memory_set_write_xor_execute(
    context: &RuntimeCallContext,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { memory_native::destack_memory_set_write_xor_execute(context, enabled) }
}
