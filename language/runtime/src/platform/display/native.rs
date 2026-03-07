use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::BindingCallContext;

#[cfg(unix)]
pub(crate) use super::unix::*;
#[cfg(windows)]
pub(crate) use super::windows::*;

#[cfg(not(any(unix, windows)))]
pub(crate) use super::unsupported::*;

/// List display backends that are available for the active target.
pub(crate) unsafe fn destack_display_backend_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayBackendDescriptor>,
) -> RuntimeResult<()> {
    // validate the output pointer first
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve backend descriptors for the current host
    let descriptors = display_backend_descriptors(binding);

    // write the stored slice to the output pointer
    unsafe {
        *out = binding.store_slice(descriptors);
    }

    Ok(())
}

/// Build backend descriptors for the current target.
fn display_backend_descriptors(binding: &BindingCallContext) -> Vec<DisplayBackendDescriptor> {
    #[cfg(unix)]
    {
        let descriptors = super::unix::display_backend_descriptors(binding);
        if !descriptors.is_empty() {
            return descriptors;
        }
    }

    #[cfg(windows)]
    {
        let descriptors = super::windows::display_backend_descriptors(binding);
        if !descriptors.is_empty() {
            return descriptors;
        }
    }

    vec![DisplayBackendDescriptor {
        backend: DisplayBackend::Null,
        name: binding.store_string("null"),
        available: false,
        priority: 0,
        capability_flags: DisplayBackendCapabilityFlags(0),
    }]
}
