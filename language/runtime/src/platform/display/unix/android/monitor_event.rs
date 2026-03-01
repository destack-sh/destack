use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayEvent, DisplayMonitorEventOpenOptions, unsupported as display_unsupported,
};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

pub(crate) unsafe fn destack_display_monitor_event_close(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_event_close(context, handle) }
}

pub(crate) unsafe fn destack_display_monitor_event_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_event_open(context, out, options) }
}

pub(crate) unsafe fn destack_display_monitor_event_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        display_unsupported::destack_display_monitor_event_read(context, out, handle, timeoutns)
    }
}

pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        display_unsupported::destack_display_monitor_event_read_batch(
            context, out, handle, maxevents, timeoutns,
        )
    }
}

pub(crate) unsafe fn destack_display_monitor_event_try_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { display_unsupported::destack_display_monitor_event_try_read(context, out, handle) }
}

pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    unsafe {
        display_unsupported::destack_display_monitor_event_try_read_batch(
            context, out, handle, maxevents,
        )
    }
}
