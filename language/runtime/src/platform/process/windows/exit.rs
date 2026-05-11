#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeResult;

use crate::runtime::BindingCallContext;

/// Exit the current process with the given code.
pub(crate) unsafe fn destack_process_exit(
    _binding: &BindingCallContext,
    code: u32,
) -> RuntimeResult<()> {
    unsafe { windows_sys::Win32::System::Threading::ExitProcess(code) }

    #[allow(unreachable_code)]
    Ok(())
}
