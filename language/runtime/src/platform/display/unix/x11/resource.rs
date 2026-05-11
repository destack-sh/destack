use std::sync::{Arc, Mutex};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as XprotoConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::resolve::resolve_payload;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;

use super::core;
use super::event::{MonitorEventStream, WindowEventStream};
use super::model::{X11DisplayHostState, X11WindowHostState};

/// Finalizer payload that destroys one x11 window id.
#[derive(Debug)]
pub(crate) struct X11WindowFinalizer {
    /// Shared x11 connection lane.
    pub(crate) connection: Arc<core::X11ConnectionState>,
    /// Native x11 window id.
    pub(crate) window: u32,
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
pub(crate) fn open_display_handle(
    binding: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_payload(X11DisplayHostState { id });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::DisplayHandle(resource_id)
}

/// Resolve one monitor identifier from one opened display handle.
pub(crate) fn resolve_display_id(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let resolved_display_state = resolve_payload::<X11DisplayHostState>(
        binding,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| core::display_not_found(operation, handle))?;

    Ok(resolved_display_state.id)
}

/// Validate that one display handle resolves to one x11 display host state.
pub(crate) fn ensure_display_handle_exists(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_display_id(context, handle, operation)?;

    Ok(())
}

/// Resolve one window host-state payload from one opened window handle.
pub(crate) fn resolve_window_host_state(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<X11WindowHostState>>> {
    resolve_payload::<Arc<Mutex<X11WindowHostState>>>(
        binding,
        window.0,
        ResourceKind::Window,
        Some(core::WINDOW_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| core::window_not_found(operation, window))
}

/// Validate that one window handle resolves to one x11 window host state.
pub(crate) fn ensure_window_handle_exists(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_host_state(context, window, operation)?;

    Ok(())
}

/// Resolve one monitor-event stream payload from one opened monitor-event handle.
pub(crate) fn resolve_monitor_event_stream(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MonitorEventStream>> {
    resolve_payload::<Arc<MonitorEventStream>>(
        binding,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_EVENT_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display event handle {} was not found", handle.0.0),
        )
    })
}

/// Validate that one monitor-event handle resolves to one x11 event stream.
pub(crate) fn ensure_monitor_event_handle_exists(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_monitor_event_stream(context, handle, operation)?;

    Ok(())
}

/// Resolve one window-event stream payload from one opened window-event handle.
pub(crate) fn resolve_window_event_stream(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowEventStream>> {
    resolve_payload::<Arc<WindowEventStream>>(
        binding,
        handle.0,
        ResourceKind::Window,
        Some(core::WINDOW_EVENT_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("window event handle {} was not found", handle.0.0),
        )
    })
}

/// Validate that one window-event handle resolves to one x11 event stream.
pub(crate) fn ensure_window_event_handle_exists(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_event_stream(context, handle, operation)?;

    Ok(())
}

/// Build one resource entry for one opened x11 window host state.
pub(crate) fn window_resource_entry(
    connection: Arc<core::X11ConnectionState>,
    window_host_state: Arc<Mutex<X11WindowHostState>>,
) -> ResourceEntry {
    let window_id = {
        let window_host_state_guard = window_host_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_host_state_guard.window
    };

    ResourceEntry::new(ResourceKind::Window)
        .with_label(core::WINDOW_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_payload(window_host_state)
        .with_finalizer(X11WindowFinalizer {
            connection,
            window: window_id,
        })
}
