#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use crate::runtime::BindingCallContext;

/// Set process file-creation umask and return the previous value.
pub(crate) unsafe fn destack_process_umask(
    _binding: &BindingCallContext,
    out: *mut u32,
    mask: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let previous = unsafe { libc::umask(mask as libc::mode_t) as u32 };
    unsafe {
        *out = previous;
    }

    Ok(())
}
