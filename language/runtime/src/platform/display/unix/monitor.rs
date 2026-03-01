#[cfg(target_os = "android")]
use super::android::monitor as backend_monitor;
#[cfg(target_os = "ios")]
use super::ios::monitor as backend_monitor;
#[cfg(target_os = "linux")]
use super::linux_x11::monitor as backend_monitor;
#[cfg(target_os = "macos")]
use super::macos::monitor as backend_monitor;
#[cfg(all(
    unix,
    not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))
))]
use super::other::monitor as backend_monitor;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::{DisplayDescriptor, DisplayMode};
use crate::platform::{NativeSlice, NativeStringRef, resource};
use crate::runtime::BindingCallContext;

/// Close one display endpoint.
pub(crate) unsafe fn destack_display_monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_close(context, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn destack_display_monitor_closest_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    unsafe {
        backend_monitor::destack_display_monitor_closest_mode(context, out, handle, requested)
    }
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_current_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_current_mode(context, out, handle) }
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn destack_display_monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_descriptor(context, out, handle) }
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_desktop_mode(context, out, handle) }
}

/// List available displays.
pub(crate) unsafe fn destack_display_monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_list(context, out) }
}

/// Read available display modes.
pub(crate) unsafe fn destack_display_monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_modes(context, out, handle) }
}

/// Open one display endpoint.
pub(crate) unsafe fn destack_display_monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_open(context, out, id) }
}

/// Read the current primary display handle.
pub(crate) unsafe fn destack_display_monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_primary(context, out) }
}

/// Apply one display mode.
pub(crate) unsafe fn destack_display_monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    unsafe { backend_monitor::destack_display_monitor_set_mode(context, handle, mode) }
}
