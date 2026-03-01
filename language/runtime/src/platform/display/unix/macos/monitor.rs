use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayDescriptor, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
    unsupported as display_unsupported,
};
use crate::platform::{NativeSlice, NativeStringRef, resource};
use crate::runtime::BindingCallContext;

pub(crate) unsafe fn destack_display_monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_close(context, handle) }
}

pub(crate) unsafe fn destack_display_monitor_closest_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    unsafe {
        display_unsupported::destack_display_monitor_closest_mode(context, out, handle, requested)
    }
}

pub(crate) unsafe fn destack_display_monitor_current_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_current_mode(context, out, handle) }
}

pub(crate) unsafe fn destack_display_monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_descriptor(context, out, handle) }
}

pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_desktop_mode(context, out, handle) }
}

pub(crate) unsafe fn destack_display_monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_list(context, out, request) }
}

pub(crate) unsafe fn destack_display_monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_modes(context, out, handle) }
}

pub(crate) unsafe fn destack_display_monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_open(context, out, id, options) }
}

pub(crate) unsafe fn destack_display_monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_primary(context, out, request) }
}

pub(crate) unsafe fn destack_display_monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_set_mode(context, handle, mode) }
}
