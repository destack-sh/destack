#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeResult;

use crate::runtime::BindingCallContext;

/// Exit the current process with the given code.
///
/// Terminate the current process without returning to the caller.
/// Exit code interpretation is host-defined and propagated to the parent process.
///
/// # Platform
/// Unix and Windows.
/// Uses _exit(2) or exit(3) on Unix and ExitProcess on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_exit(
    _binding: &BindingCallContext,
    code: u32,
) -> RuntimeResult<()> {
    unsafe { windows_sys::Win32::System::Threading::ExitProcess(code) }

    #[allow(unreachable_code)]
    Ok(())
}
