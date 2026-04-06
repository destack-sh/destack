use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::BackendSupport;
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
    DisplayDragBeginOptions, DisplayDragOperation,
};
use crate::platform::fs::OsPath;
use crate::platform::resource::DisplayDragSessionHandle;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::BindingCallContext;

use super::core;

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
        support: BackendSupport::UnsupportedTarget,
        priority: 0,
        capability_flags: DisplayBackendCapabilityFlags(0),
    }]
}

/// Begin one host drag session.
pub(crate) unsafe fn destack_display_drag_begin(
    _binding: &BindingCallContext,
    out: *mut DisplayDragOperation,
    options: DisplayDragBeginOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.drag.begin")).boxed())
}

/// Set one host drag session operation.
pub(crate) unsafe fn destack_display_drag_session_set_operation(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    operation: DisplayDragOperation,
) -> RuntimeResult<()> {
    core::set_drag_session_operation(binding, session, operation)
}

/// Close one host drag session.
pub(crate) unsafe fn destack_display_drag_session_close(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
) -> RuntimeResult<()> {
    core::close_drag_session(binding, session)
}

/// Read one host drag session item as bytes.
pub(crate) unsafe fn destack_display_drag_session_read_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    session: DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<()> {
    // validate the output pointer first
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stored item payload and hand it back as a runtime slice
    let bytes = core::read_drag_session_item_bytes(binding, session, itemindex)?;
    let bytes = binding.store_slice(bytes);

    unsafe {
        out.write(bytes);
    }

    Ok(())
}

/// Read one host drag session item as one path.
pub(crate) unsafe fn destack_display_drag_session_read_path(
    binding: &BindingCallContext,
    out: *mut OsPath,
    session: DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<()> {
    // validate the output pointer first
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stored path payload and write it directly
    let path = core::read_drag_session_item_path(binding, session, itemindex)?;

    unsafe {
        out.write(path);
    }

    Ok(())
}

/// Read one host drag session item as text.
pub(crate) unsafe fn destack_display_drag_session_read_text(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    session: DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<()> {
    // validate the output pointer first
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stored text payload and intern it for the caller
    let text = core::read_drag_session_item_text(binding, session, itemindex)?;
    let text = binding.store_string(&text);

    unsafe {
        out.write(text);
    }

    Ok(())
}
