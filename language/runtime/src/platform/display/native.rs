#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::display::{DisplayInfo, DisplayMode, WindowEvent, WindowOptions};
use crate::platform::resource;

/// Stub for destack.display.monitor.close.
pub unsafe fn destack_display_close(
    context: &RuntimeCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.close",
    ))
    .boxed())
}

/// Stub for destack.display.monitor.list.
pub unsafe fn destack_display_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<DisplayInfo>,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.list")).boxed())
}

/// Stub for destack.display.monitor.modes.
pub unsafe fn destack_display_modes(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_MODES)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.modes",
    ))
    .boxed())
}

/// Stub for destack.display.monitor.open.
pub unsafe fn destack_display_open(
    context: &RuntimeCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.open")).boxed())
}

/// Stub for destack.display.monitor.setMode.
pub unsafe fn destack_display_set_mode(
    context: &RuntimeCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_SET_MODE)?;
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setMode",
    ))
    .boxed())
}

/// Stub for destack.display.window.close.
pub unsafe fn destack_display_window_close(
    context: &RuntimeCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_CLOSE)?;
    let _ = window;

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.close")).boxed())
}

/// Stub for destack.display.window.event.
pub unsafe fn destack_display_window_event(
    context: &RuntimeCallContext,
    out: *mut WindowEvent,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_EVENT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, window);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.event")).boxed())
}

/// Stub for destack.display.window.open.
pub unsafe fn destack_display_window_open(
    context: &RuntimeCallContext,
    out: *mut resource::WindowHandle,
    display: resource::DisplayHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, display, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.open")).boxed())
}

/// Stub for destack.display.window.setTitle.
pub unsafe fn destack_display_window_set_title(
    context: &RuntimeCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_SET_TITLE)?;
    let _ = (window, title);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTitle",
    ))
    .boxed())
}

/// Stub for destack.display.window.tryEvent.
pub unsafe fn destack_display_window_try_event(
    context: &RuntimeCallContext,
    out: *mut WindowEvent,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_TRY_EVENT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.tryEvent",
    ))
    .boxed())
}

/// Stub for destack.display.window.vsyncWait.
pub unsafe fn destack_display_window_vsync_wait(
    context: &RuntimeCallContext,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_VSYNC_WAIT)?;
    let _ = (window, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.vsyncWait",
    ))
    .boxed())
}
