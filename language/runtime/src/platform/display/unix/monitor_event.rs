#[cfg(target_os = "android")]
use super::android::monitor_event as backend_monitor_event;
#[cfg(target_os = "ios")]
use super::ios::monitor_event as backend_monitor_event;
#[cfg(target_os = "linux")]
use super::linux_x11::monitor_event as backend_monitor_event;
#[cfg(target_os = "macos")]
use super::macos::monitor_event as backend_monitor_event;
#[cfg(all(
    unix,
    not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))
))]
use super::other::monitor_event as backend_monitor_event;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayEvent;
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Close one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_close(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor_event::destack_display_monitor_event_close(context, handle) }
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor_event::destack_display_monitor_event_open(context, out) }
}

/// Wait for one monitor event.
pub(crate) unsafe fn destack_display_monitor_event_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    unsafe { backend_monitor_event::destack_display_monitor_event_read(context, out, handle, timeout_ns) }
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    unsafe {
        backend_monitor_event::destack_display_monitor_event_read_batch(
            context, out, handle, max_events, timeout_ns,
        )
    }
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { backend_monitor_event::destack_display_monitor_event_try_read(context, out, handle) }
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
) -> RuntimeResult<()> {
    unsafe {
        backend_monitor_event::destack_display_monitor_event_try_read_batch(
            context, out, handle, max_events,
        )
    }
}
