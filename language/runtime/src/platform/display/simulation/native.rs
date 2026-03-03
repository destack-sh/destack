use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::display::DisplayBackendDescriptor;
use crate::runtime::{BindingCallContext, NativeSlice};

/// List display backends that are available for the active target.
pub(crate) unsafe fn destack_display_backend_list(
    _context: &BindingCallContext,
    out: *mut NativeSlice<DisplayBackendDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.backend.list")).boxed())
}

pub(crate) use crate::platform::display::unsupported::*;
