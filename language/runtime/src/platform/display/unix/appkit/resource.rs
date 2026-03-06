use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::bindings::BindingEngine;

use super::core::{self, AppKitRuntimeState};
use super::event::{MonitorEventBinding, WindowEventBinding};
use super::model::{AppKitDisplayBinding, AppKitWindowBinding};

/// Resolve one typed resource payload by kind and optional label.
fn resolve_payload<T: Clone + Send + Sync + 'static>(
    context: &BindingCallContext,
    handle: resource::ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
) -> Option<T> {
    context
        .agent()
        .resources
        .with_entry(handle, |entry| {
            // reject mismatched resource kinds
            if entry.kind != kind {
                return None;
            }

            // reject mismatched resource labels
            if let Some(expected_label) = label
                && entry.label.as_deref() != Some(expected_label)
            {
                return None;
            }

            entry.payload_cloned::<T>()
        })
        .flatten()
}

/// Insert one monitor resource for one monitor identifier.
pub(crate) fn open_display_handle(
    context: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_payload(AppKitDisplayBinding { id });
    let resource_id = context
        .agent()
        .resources
        .insert(entry, Some(context.engine()));

    resource::DisplayHandle(resource_id)
}

/// Insert one monitor resource from callback-owned runtime state.
pub(crate) fn open_display_handle_for_runtime(
    runtime_state: &Arc<AppKitRuntimeState>,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_payload(AppKitDisplayBinding { id });
    let resource_id =
        core::resource_table(runtime_state).insert(entry, Some(BindingEngine::Native));

    resource::DisplayHandle(resource_id)
}

/// Resolve one cached display handle for one stable monitor identifier from runtime state.
pub(crate) fn cached_display_handle_for_runtime(
    runtime_state: &Arc<AppKitRuntimeState>,
    id: &str,
) -> resource::DisplayHandle {
    let mut cache = runtime_state
        .display_handle_cache
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // reuse one still-live resource handle when possible
    if let Some(handle) = cache.get(id).copied()
        && core::resource_table(runtime_state).contains(handle.0)
    {
        return handle;
    }

    let handle = open_display_handle_for_runtime(runtime_state, id.to_string());
    cache.insert(id.to_string(), handle);
    handle
}

/// Resolve one monitor identifier from one opened display handle.
pub(crate) fn resolve_display_id(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let binding = resolve_payload::<AppKitDisplayBinding>(
        context,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_RESOURCE_LABEL),
    )
    .ok_or_else(|| core::display_not_found(operation, handle))?;

    Ok(binding.id)
}

/// Build one resource entry for one opened AppKit window binding.
pub(crate) fn window_resource_entry(binding: Arc<Mutex<AppKitWindowBinding>>) -> ResourceEntry {
    ResourceEntry::new(ResourceKind::Window)
        .with_label(core::WINDOW_RESOURCE_LABEL)
        .with_payload(binding)
}

/// Resolve one window binding payload from one opened window handle.
pub(crate) fn resolve_window_binding(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<AppKitWindowBinding>>> {
    resolve_payload::<Arc<Mutex<AppKitWindowBinding>>>(
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
