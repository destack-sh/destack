use crate::diagnostic::{RuntimeErrorId, RuntimeResult};
use crate::platform::error::PlatformError;
use crate::platform::error::core::{NativeStringStore, platform_error_native, take_platform_error};
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
pub unsafe fn destack_error_take_platform_error(
    context: &BindingCallContext,
    out: *mut PlatformError,
    error_id: u64,
) -> RuntimeResult<()> {
    let error = take_platform_error(
        &context.runtime().errors,
        RuntimeErrorId::from_raw(error_id),
    );
    let store = NativeStringStore::new(context);
    let platform_error = platform_error_native(&store, &error);

    unsafe {
        out.write(platform_error);
    }

    Ok(())
}
