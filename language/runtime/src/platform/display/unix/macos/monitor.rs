use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayDescriptor, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
    unsupported,
};
use crate::platform::resource;
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

/// Close one display endpoint.
pub(in crate::platform::display::host::unix) unsafe fn monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_close(context, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(in crate::platform::display::host::unix) unsafe fn monitor_closest_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_closest_mode(context, out, handle, requested) }
}

/// Read the current mode for one opened display.
pub(in crate::platform::display::host::unix) unsafe fn monitor_current_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_current_mode(context, out, handle) }
}

/// Read descriptor metadata for one opened display.
pub(in crate::platform::display::host::unix) unsafe fn monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_descriptor(context, out, handle) }
}

/// Read the desktop-preferred mode for one opened display.
pub(in crate::platform::display::host::unix) unsafe fn monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_desktop_mode(context, out, handle) }
}

/// List available displays.
pub(in crate::platform::display::host::unix) unsafe fn monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_list(context, out, request) }
}

/// Read available display modes.
pub(in crate::platform::display::host::unix) unsafe fn monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_modes(context, out, handle) }
}

/// Open one display endpoint.
pub(in crate::platform::display::host::unix) unsafe fn monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_open(context, out, id, options) }
}

/// Read the current primary display handle.
pub(in crate::platform::display::host::unix) unsafe fn monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_primary(context, out, request) }
}

/// Apply one display mode.
pub(in crate::platform::display::host::unix) unsafe fn monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_set_mode(context, handle, mode) }
}
