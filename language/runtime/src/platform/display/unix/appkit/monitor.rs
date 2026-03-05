use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayColorState, DisplayDescriptor, DisplayGammaRamp, DisplayHdrMode, DisplayMode,
    DisplayMonitorListRequest, DisplayMonitorOpenOptions, unsupported,
};
use crate::platform::resource;
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

/// Close one display endpoint.
pub(in crate::platform::display::host::unix) unsafe fn monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_close(binding, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(in crate::platform::display::host::unix) unsafe fn monitor_closest_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_closest_mode(binding, out, handle, requested) }
}

/// Read the current mode for one opened display.
pub(in crate::platform::display::host::unix) unsafe fn monitor_current_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_current_mode(binding, out, handle) }
}

/// Read descriptor metadata for one opened display.
pub(in crate::platform::display::host::unix) unsafe fn monitor_descriptor(
    binding: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_descriptor(binding, out, handle) }
}

/// Read the desktop-preferred mode for one opened display.
pub(in crate::platform::display::host::unix) unsafe fn monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_desktop_mode(binding, out, handle) }
}

/// List available displays.
pub(in crate::platform::display::host::unix) unsafe fn monitor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_list(binding, out, request) }
}

/// Read available display modes.
pub(in crate::platform::display::host::unix) unsafe fn monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_modes(binding, out, handle) }
}

/// Open one display endpoint.
pub(in crate::platform::display::host::unix) unsafe fn monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_open(binding, out, id, options) }
}

/// Read the current primary display handle.
pub(in crate::platform::display::host::unix) unsafe fn monitor_primary(
    binding: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_primary(binding, out, request) }
}

/// Apply one display mode.
pub(in crate::platform::display::host::unix) unsafe fn monitor_set_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_set_mode(binding, handle, mode) }
}

/// Read display color state.
pub(in crate::platform::display::host::unix) unsafe fn monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_color_state(binding, out, handle) }
}

/// Read display HDR mode.
pub(in crate::platform::display::host::unix) unsafe fn monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_hdr_mode(binding, out, handle) }
}

/// Set display HDR mode.
pub(in crate::platform::display::host::unix) unsafe fn monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_set_hdr_mode(binding, handle, mode) }
}

/// Read display gamma ramp.
pub(in crate::platform::display::host::unix) unsafe fn monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_gamma_ramp(binding, out, handle) }
}

/// Set display gamma ramp.
pub(in crate::platform::display::host::unix) unsafe fn monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_set_gamma_ramp(binding, handle, ramp) }
}
