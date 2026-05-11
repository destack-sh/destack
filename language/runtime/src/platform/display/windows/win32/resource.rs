use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, PostMessageW, WM_CLOSE};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::resolve::resolve_payload;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::binding::BindingAffinity;

use super::constants::*;
use super::core;
use super::event::{MonitorEventStream, WindowEventStream};
use super::model::{Win32DisplayHostState, Win32WindowHostState};

/// Finalizer payload that destroys one Win32 window handle.
#[derive(Debug)]
pub(crate) struct Win32WindowFinalizer {
    /// Native Win32 window handle.
    pub(crate) hwnd: HWND,
}

impl ResourceFinalizer for Win32WindowFinalizer {
    /// Close one window handle during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        // destroy the native window when it is still alive
        if unsafe { IsWindow(self.hwnd) } != 0 {
            // route forced teardown onto the window event-loop context
            unsafe {
                let _ = PostMessageW(self.hwnd, WM_CLOSE, core::WINDOW_CLOSE_FORCE_WPARAM, 0);
            }
        }
    }
}

/// Insert one monitor resource for one monitor identifier.
pub(crate) fn open_display_handle(
    binding: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(DISPLAY_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_payload(Win32DisplayHostState { id });
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
    let resolved_display_state = resolve_payload::<Win32DisplayHostState>(
        binding,
        handle.0,
        ResourceKind::Display,
        Some(DISPLAY_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display handle {} was not found", handle.0.0),
        )
    })?;

    Ok(resolved_display_state.id)
}

/// Resolve one window host-state payload from one opened window handle.
pub(crate) fn resolve_window_host_state(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<Win32WindowHostState>>> {
    resolve_payload::<Arc<Mutex<Win32WindowHostState>>>(
        binding,
        window.0,
        ResourceKind::Window,
        Some(WINDOW_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("window handle {} was not found", window.0.0),
        )
    })
}

/// Validate that one window handle resolves to one win32 window host state.
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
        Some(DISPLAY_EVENT_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display event handle {} was not found", handle.0.0),
        )
    })
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
        Some(WINDOW_EVENT_RESOURCE_LABEL),
        operation,
    )?
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("window event handle {} was not found", handle.0.0),
        )
    })
}

/// Build one resource entry for one opened window host state.
pub(crate) fn window_resource_entry(
    hwnd: HWND,
    binding: Arc<Mutex<Win32WindowHostState>>,
) -> ResourceEntry {
    ResourceEntry::new(ResourceKind::Window)
        .with_label(WINDOW_RESOURCE_LABEL)
        .with_binding_affinity(BindingAffinity::Worker)
        .with_handle(hwnd as *mut c_void)
        .with_payload(binding)
        .with_finalizer(Win32WindowFinalizer { hwnd })
}
