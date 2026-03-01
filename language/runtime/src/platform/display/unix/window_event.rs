#[cfg(target_os = "android")]
use super::android as backend_window_event;
#[cfg(target_os = "ios")]
use super::ios as backend_window_event;
#[cfg(target_os = "linux")]
use super::linux_x11 as backend_window_event;
#[cfg(target_os = "macos")]
use super::macos as backend_window_event;
#[cfg(all(
    unix,
    not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))
))]
use super::other as backend_window_event;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowEvent, WindowEventOpenOptions};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Close one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_close(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { backend_window_event::destack_display_window_event_close(context, handle) }
}

/// Open one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_open(
    context: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    unsafe { backend_window_event::destack_display_window_event_open(context, out, options) }
}

/// Wait for one window event.
pub(crate) unsafe fn destack_display_window_event_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        backend_window_event::destack_display_window_event_read(context, out, handle, timeoutns)
    }
}

/// Wait for one batch of window events.
pub(crate) unsafe fn destack_display_window_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        backend_window_event::destack_display_window_event_read_batch(
            context, out, handle, maxevents, timeoutns,
        )
    }
}

/// Poll one window event without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { backend_window_event::destack_display_window_event_try_read(context, out, handle) }
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    unsafe {
        backend_window_event::destack_display_window_event_try_read_batch(
            context, out, handle, maxevents,
        )
    }
}
