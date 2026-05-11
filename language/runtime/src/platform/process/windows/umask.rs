#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use crate::runtime::BindingCallContext;

/// Set process file-creation umask and return the previous value.
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
