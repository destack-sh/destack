use std::sync::{Arc, Mutex};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as XprotoConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{
    ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, resolve_payload,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core;
use super::event::{MonitorEventBinding, WindowEventBinding};
use super::model::{X11DisplayBinding, X11WindowBinding};

/// Finalizer payload that destroys one x11 window id.
#[derive(Debug)]
pub(super) struct X11WindowFinalizer {
    /// Shared x11 connection lane.
    pub(super) connection: Arc<core::X11ConnectionState>,
    /// Native x11 window id.
    pub(super) window: u32,
}

impl ResourceFinalizer for X11WindowFinalizer {
    /// Close one window handle during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        // destroy the native window and flush any pending requests
        let _ = self.connection.connection.destroy_window(self.window);
        let _ = self.connection.connection.flush();
    }
}

/// Insert one monitor resource for one monitor identifier.
pub(super) fn open_display_handle(
    context: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_payload(X11DisplayBinding { id });
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

    resource::DisplayHandle(resource_id)
}

/// Resolve one monitor identifier from one opened display handle.
pub(super) fn resolve_display_id(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let binding = resolve_payload::<X11DisplayBinding>(
        context,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_RESOURCE_LABEL),
    )
    .ok_or_else(|| core::display_not_found(operation, handle))?;

    Ok(binding.id)
}

/// Resolve one window binding payload from one opened window handle.
pub(super) fn resolve_window_binding(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<X11WindowBinding>>> {
    resolve_payload::<Arc<Mutex<X11WindowBinding>>>(
        context,
        window.0,
        ResourceKind::Window,
        Some(core::WINDOW_RESOURCE_LABEL),
    )
    .ok_or_else(|| core::window_not_found(operation, window))
}

/// Resolve one monitor-event binding payload from one opened monitor-event handle.
pub(super) fn resolve_monitor_event_binding(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MonitorEventBinding>> {
    resolve_payload::<Arc<MonitorEventBinding>>(
        context,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_EVENT_RESOURCE_LABEL),
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display event handle {} was not found", handle.0.0),
        )
    })
}

/// Resolve one window-event binding payload from one opened window-event handle.
pub(super) fn resolve_window_event_binding(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowEventBinding>> {
    resolve_payload::<Arc<WindowEventBinding>>(
        context,
        handle.0,
        ResourceKind::Window,
        Some(core::WINDOW_EVENT_RESOURCE_LABEL),
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("window event handle {} was not found", handle.0.0),
        )
    })
}

/// Build one resource entry for one opened x11 window binding.
pub(super) fn window_resource_entry(
    connection: Arc<core::X11ConnectionState>,
    binding: Arc<Mutex<X11WindowBinding>>,
) -> ResourceEntry {
    let window_id = {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        binding.window
    };

    ResourceEntry::new(ResourceKind::Window)
        .with_label(core::WINDOW_RESOURCE_LABEL)
        .with_payload(binding)
        .with_finalizer(X11WindowFinalizer {
            connection,
            window: window_id,
        })
}
