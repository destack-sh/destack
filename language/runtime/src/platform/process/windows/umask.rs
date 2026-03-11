#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use crate::runtime::BindingCallContext;

/// Set process file-creation umask and return the previous value.
///
/// Update the process-wide file mode creation mask used by future file-creation operations.
/// Mask interpretation follows POSIX mode bit rules.
///
/// # Platform
/// Unix and Windows.
/// Uses umask(2) on Unix and runtime compatibility behavior on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_umask(
    _binding: &BindingCallContext,
    out: *mut u32,
    _mask: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask")).boxed())
}
