use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{ResourceEntry, ResourceKind, resolve_payload};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core;
use super::event::{MonitorEventBinding, WindowEventBinding};
use super::model::{WaylandDisplayBinding, WaylandWindowBinding};

/// Insert one monitor resource for one monitor identifier.
pub(crate) fn open_display_handle(
    context: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_payload(WaylandDisplayBinding { id });
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

    resource::DisplayHandle(resource_id)
}

/// Resolve one monitor identifier from one opened display handle.
pub(crate) fn resolve_display_id(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let binding = resolve_payload::<WaylandDisplayBinding>(
        context,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_RESOURCE_LABEL),
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display handle {} was not found", handle.0.0),
        )
    })?;

    Ok(binding.id)
}

/// Validate that one display handle resolves to one wayland display binding.
pub(crate) fn ensure_display_binding_exists(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_display_id(context, handle, operation)?;

    Ok(())
}

/// Build one resource entry for one opened wayland window binding.
pub(crate) fn window_resource_entry(binding: Arc<Mutex<WaylandWindowBinding>>) -> ResourceEntry {
    ResourceEntry::new(ResourceKind::Window)
        .with_label(core::WINDOW_RESOURCE_LABEL)
        .with_payload(binding)
}

/// Resolve one window binding payload from one opened window handle.
pub(crate) fn resolve_window_binding(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<WaylandWindowBinding>>> {
    resolve_payload::<Arc<Mutex<WaylandWindowBinding>>>(
        context,
        window.0,
        ResourceKind::Window,
        Some(core::WINDOW_RESOURCE_LABEL),
    )
    .ok_or_else(|| core::window_not_found(operation, window))
}

/// Resolve one monitor-event binding payload from one opened monitor-event handle.
pub(crate) fn resolve_monitor_event_binding(
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

/// Validate that one monitor-event handle resolves to one wayland event binding.
pub(crate) fn ensure_monitor_event_binding_exists(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_monitor_event_binding(context, handle, operation)?;

    Ok(())
}

/// Validate that one window handle resolves to one wayland window binding.
pub(crate) fn ensure_window_binding_exists(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_binding(context, window, operation)?;

    Ok(())
}

/// Resolve one window-event binding payload from one opened window-event handle.
pub(crate) fn resolve_window_event_binding(
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

/// Validate that one window-event handle resolves to one wayland event binding.
pub(crate) fn ensure_window_event_binding_exists(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_event_binding(context, handle, operation)?;

    Ok(())
}
