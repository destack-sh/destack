#![allow(dead_code)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackendDescriptor, DisplayDragBeginOptions, DisplayDragOperation,
};
use crate::platform::fs::OsPath;
use crate::platform::resource::DisplayDragSessionHandle;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::BindingCallContext;

/// List display backends that are available for the active target.
pub(crate) unsafe fn destack_display_backend_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayBackendDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.backend.list")).boxed())
}

/// Begin one drag session.
pub(crate) unsafe fn destack_display_drag_begin(
    _binding: &BindingCallContext,
    out: *mut DisplayDragOperation,
    options: DisplayDragBeginOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.drag.begin")).boxed())
}

/// Set one drag session operation.
pub(crate) unsafe fn destack_display_drag_session_set_operation(
    _binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    operation: DisplayDragOperation,
) -> RuntimeResult<()> {
    let _ = (session, operation);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionSetOperation",
    ))
    .boxed())
}

/// Close one drag session.
pub(crate) unsafe fn destack_display_drag_session_close(
    _binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
) -> RuntimeResult<()> {
    let _ = session;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionClose",
    ))
    .boxed())
}

/// Read one drag session item as bytes.
pub(crate) unsafe fn destack_display_drag_session_read_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    session: DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<()> {
    let _ = (out, session, itemindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionReadBytes",
    ))
    .boxed())
}

/// Read one drag session item as one path.
pub(crate) unsafe fn destack_display_drag_session_read_path(
    _binding: &BindingCallContext,
    out: *mut OsPath,
    session: DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<()> {
    let _ = (out, session, itemindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionReadPath",
    ))
    .boxed())
}

/// Read one drag session item as text.
pub(crate) unsafe fn destack_display_drag_session_read_text(
    _binding: &BindingCallContext,
    out: *mut NativeStringRef,
    session: DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<()> {
    let _ = (out, session, itemindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionReadText",
    ))
    .boxed())
}

pub(crate) use crate::platform::display::unsupported::*;
