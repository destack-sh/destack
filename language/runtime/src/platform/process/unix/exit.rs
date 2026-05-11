#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeResult;

use crate::runtime::BindingCallContext;

/// Exit the current process with the given code.
pub(crate) unsafe fn destack_process_exit(
    _binding: &BindingCallContext,
    code: u32,
) -> RuntimeResult<()> {
    unsafe { libc::_exit(code as i32) }
}
