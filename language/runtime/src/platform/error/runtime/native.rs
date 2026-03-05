use crate::diagnostic::RuntimeResult;
use crate::platform::error::{PlatformError, native as error_native};
use crate::runtime::BindingCallContext;

/// Take a runtime platform error by id.
///
/// Reads and consumes one stored platform error entry by id.
/// Removes the entry from the runtime error store after a successful read.
///
/// # Platform
/// Managed by the runtime platform layer rather than an OS-specific syscall.
/// Uses no direct syscall: reads and clears one runtime error-store entry.
///
/// # Errors
/// Returns `invalidArgument` or `generic`.
///
/// # Security
/// Requires `diagnostic.read`.
///
/// # Replay
/// Deterministic.
pub(crate) unsafe fn destack_error_take_platform_error(
    binding: &BindingCallContext,
    out: *mut PlatformError,
    error_id: u64,
) -> RuntimeResult<()> {
    unsafe { error_native::destack_error_take_platform_error(binding, out, error_id) }
}
