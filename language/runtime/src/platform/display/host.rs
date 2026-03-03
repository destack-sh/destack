#[cfg(unix)]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(not(any(unix, windows)))]
pub(crate) use super::unsupported::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
};
use crate::runtime::{BindingCallContext, NativeSlice};

/// List display backends that are available for the active target.
pub(crate) unsafe fn destack_display_backend_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayBackendDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let descriptors = display_backend_descriptors(context);
    unsafe {
        *out = context.store_slice(descriptors);
    }

    Ok(())
}

/// Build backend descriptors for the current target.
fn display_backend_descriptors(context: &BindingCallContext) -> Vec<DisplayBackendDescriptor> {
    #[cfg(unix)]
    {
        let descriptors = unix::display_backend_descriptors(context);
        if !descriptors.is_empty() {
            return descriptors;
        }
    }

    #[cfg(windows)]
    {
        let descriptors = windows::display_backend_descriptors(context);
        if !descriptors.is_empty() {
            return descriptors;
        }
    }

    vec![DisplayBackendDescriptor {
        backend: DisplayBackend::Null,
        name: context.store_string("null"),
        available: false,
        priority: 0,
        capability_flags: DisplayBackendCapabilityFlags(0),
    }]
}
