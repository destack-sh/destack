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
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::BindingCallContext;

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
    let mut descriptors = Vec::new();

    #[cfg(target_os = "linux")]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::X11,
            name: context.store_string("x11"),
            available: true,
            priority: 100,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    #[cfg(target_os = "macos")]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::AppKit,
            name: context.store_string("appkit"),
            available: true,
            priority: 100,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    #[cfg(target_os = "windows")]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::Win32,
            name: context.store_string("win32"),
            available: true,
            priority: 100,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    #[cfg(target_os = "android")]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::Android,
            name: context.store_string("android"),
            available: true,
            priority: 100,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    #[cfg(target_os = "ios")]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::UIKit,
            name: context.store_string("uikit"),
            available: true,
            priority: 100,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    #[cfg(all(
        unix,
        not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "linux",
            target_os = "macos"
        ))
    ))]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::Null,
            name: context.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        descriptors.push(DisplayBackendDescriptor {
            backend: DisplayBackend::Null,
            name: context.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: DisplayBackendCapabilityFlags(0),
        });
    }

    descriptors
}
