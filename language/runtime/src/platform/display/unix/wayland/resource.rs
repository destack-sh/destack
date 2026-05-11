use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::resolve::resolve_payload;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::bindings::BindingAffinity;

use super::core;
use super::event::{MonitorEventStream, WindowEventStream};
use super::model::{WaylandDisplayHostState, WaylandWindowHostState};

/// Insert one monitor resource for one monitor identifier.
pub(crate) fn open_display_handle(
    context: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_payload(WaylandDisplayHostState { id });
    let resource_id =
        context
            .worker()
            .resources
            .insert(&context.world(), entry, Some(context.engine()));

    resource::DisplayHandle(resource_id)
}

/// Resolve one monitor identifier from one opened display handle.
pub(crate) fn resolve_display_id(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let host_state = resolve_payload::<WaylandDisplayHostState>(
        context,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display handle {} was not found", handle.0.0),
        )
    })?;

    Ok(host_state.id)
}

/// Validate that one display handle resolves to one wayland display host state.
pub(crate) fn ensure_display_handle_exists(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_display_id(context, handle, operation)?;

    Ok(())
}

/// Build one resource entry for one opened wayland window host state.
pub(crate) fn window_resource_entry(binding: Arc<Mutex<WaylandWindowHostState>>) -> ResourceEntry {
    ResourceEntry::new(ResourceKind::Window)
        .with_label(core::WINDOW_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_payload(binding)
}

/// Resolve one window host-state payload from one opened window handle.
pub(crate) fn resolve_window_host_state(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<WaylandWindowHostState>>> {
    resolve_payload::<Arc<Mutex<WaylandWindowHostState>>>(
        context,
        window.0,
        ResourceKind::Window,
        Some(core::WINDOW_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| core::window_not_found(operation, window))
}

/// Resolve one monitor-event stream payload from one opened monitor-event handle.
pub(crate) fn resolve_monitor_event_stream(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MonitorEventStream>> {
    resolve_payload::<Arc<MonitorEventStream>>(
        context,
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

/// Validate that one monitor-event handle resolves to one wayland event stream.
pub(crate) fn ensure_monitor_event_handle_exists(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_monitor_event_stream(context, handle, operation)?;

    Ok(())
}

/// Validate that one window handle resolves to one wayland window host state.
pub(crate) fn ensure_window_handle_exists(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_host_state(context, window, operation)?;

    Ok(())
}

/// Resolve one window-event stream payload from one opened window-event handle.
pub(crate) fn resolve_window_event_stream(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowEventStream>> {
    resolve_payload::<Arc<WindowEventStream>>(
        context,
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

/// Validate that one window-event handle resolves to one wayland event stream.
pub(crate) fn ensure_window_event_handle_exists(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_event_stream(context, handle, operation)?;

    Ok(())
}
