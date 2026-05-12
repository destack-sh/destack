use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::resolve::resolve_payload;
use crate::platform::resource::{ResourceAffinity, ResourceEntry, ResourceKind};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;

use super::core::{self, AppKitRuntimeState};
use super::event::{MonitorEventStream, WindowEventStream};
use super::model::{AppKitDisplayHostState, AppKitWindowHostState};

/// Insert one monitor resource for one monitor identifier.
pub(crate) fn open_display_handle(
    context: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_payload(AppKitDisplayHostState { id });
    let resource_id =
        context
            .worker()
            .resources
            .insert(context.world(), entry, Some(context.engine()));

    resource::DisplayHandle(resource_id)
}

/// Insert one monitor resource from callback-owned runtime state.
pub(crate) fn open_display_handle_for_runtime(
    runtime_state: &Arc<AppKitRuntimeState>,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(core::DISPLAY_RESOURCE_LABEL)
        .with_affinity(ResourceAffinity::Worker)
        .with_payload(AppKitDisplayHostState { id });
    let resource_id = runtime_state.resource_table().insert_untracked(entry);

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
        && runtime_state.resource_table().contains(handle.0)
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
    let host_state = resolve_payload::<AppKitDisplayHostState>(
        context,
        handle.0,
        ResourceKind::Display,
        Some(core::DISPLAY_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| core::display_not_found(operation, handle))?;

    Ok(host_state.id)
}

/// Build one resource entry for one opened AppKit window host state.
pub(crate) fn window_resource_entry(binding: Arc<Mutex<AppKitWindowHostState>>) -> ResourceEntry {
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
) -> RuntimeResult<Arc<Mutex<AppKitWindowHostState>>> {
    resolve_payload::<Arc<Mutex<AppKitWindowHostState>>>(
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
            format!("display event handle {} was not found", handle.0.local_id),
        )
    })
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
            format!("window event handle {} was not found", handle.0.local_id),
        )
    })
}
