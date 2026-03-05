use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayMonitorEvent, DisplayMonitorEventOpenOptions, WindowEvent, WindowEventOpenOptions,
    unsupported,
};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Close one global monitor-event stream.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_close(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_event_close(binding, handle) }
}

/// Open one global monitor-event stream.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_event_open(binding, out, options) }
}

/// Wait for one monitor event.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_event_read(binding, out, handle, timeoutns) }
}

/// Wait for one batch of monitor events.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_monitor_event_read_batch(
            binding, out, handle, maxevents, timeoutns,
        )
    }
}

/// Poll one monitor event without blocking.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_try_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_monitor_event_try_read(binding, out, handle) }
}

/// Poll one batch of monitor events without blocking.
pub(in crate::platform::display::host::unix) unsafe fn monitor_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_monitor_event_try_read_batch(binding, out, handle, maxevents)
    }
}

/// Close one global window-event stream.
pub(in crate::platform::display::host::unix) unsafe fn window_event_close(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_event_close(binding, handle) }
}

/// Open one global window-event stream.
pub(in crate::platform::display::host::unix) unsafe fn window_event_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_event_open(binding, out, options) }
}

/// Wait for one window event.
pub(in crate::platform::display::host::unix) unsafe fn window_event_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_event_read(binding, out, handle, timeoutns) }
}

/// Wait for one batch of window events.
pub(in crate::platform::display::host::unix) unsafe fn window_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_window_event_read_batch(
            binding, out, handle, maxevents, timeoutns,
        )
    }
}

/// Poll one window event without blocking.
pub(in crate::platform::display::host::unix) unsafe fn window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_event_try_read(binding, out, handle) }
}

/// Poll one batch of window events without blocking.
pub(in crate::platform::display::host::unix) unsafe fn window_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_window_event_try_read_batch(binding, out, handle, maxevents)
    }
}
