use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetWindowThreadProcessId, IsWindow, PostMessageW, WM_CLOSE,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::runtime::BindingCallContext;

use super::core;
use super::event::{MonitorEventBinding, WindowEventBinding};
use super::model::{Win32DisplayBinding, Win32WindowBinding};

/// Resource-table label for opened display monitor handles.
pub(super) const DISPLAY_RESOURCE_LABEL: &str = "display.monitor";
/// Resource-table label for opened window handles.
pub(super) const WINDOW_RESOURCE_LABEL: &str = "display.window";
/// Resource-table label for opened monitor-event stream handles.
pub(super) const DISPLAY_EVENT_RESOURCE_LABEL: &str = "display.monitor.event";
/// Resource-table label for opened window-event stream handles.
pub(super) const WINDOW_EVENT_RESOURCE_LABEL: &str = "display.window.event";

/// Finalizer payload that destroys one Win32 window handle.
#[derive(Debug)]
pub(super) struct Win32WindowFinalizer {
    /// Native Win32 window handle.
    pub(super) hwnd: HWND,
}

impl ResourceFinalizer for Win32WindowFinalizer {
    /// Close one window handle during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        // destroy the native window when it is still alive
        if unsafe { IsWindow(self.hwnd) } != 0 {
            let owner_thread_id =
                unsafe { GetWindowThreadProcessId(self.hwnd, std::ptr::null_mut()) };
            let current_thread_id = unsafe { GetCurrentThreadId() };

            // destroy directly on owner thread, otherwise request close asynchronously
            if owner_thread_id == current_thread_id {
                unsafe {
                    DestroyWindow(self.hwnd);
                }
            } else {
                unsafe {
                    let _ = PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
                }
            }
        }
    }
}

/// Insert one monitor resource for one monitor identifier.
pub(super) fn open_display_handle(
    context: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(DISPLAY_RESOURCE_LABEL)
        .with_payload(Win32DisplayBinding { id });
    let resource_id = context.runtime().resources.insert(entry);
    resource::DisplayHandle(resource_id)
}

/// Resolve one monitor identifier from one opened display handle.
pub(super) fn resolve_display_id(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Display {
                return None;
            }

            if entry.label.as_deref() != Some(DISPLAY_RESOURCE_LABEL) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Win32DisplayBinding>())
                .map(|binding| binding.id.clone())
        })
        .flatten();

    resolved.ok_or_else(|| {
        core::not_found(
            operation,
            format!("display handle {} was not found", handle.0.0),
        )
    })
}

/// Resolve one window binding payload from one opened window handle.
pub(super) fn resolve_window_binding(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<Win32WindowBinding>>> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(window.0, |entry| {
            if entry.kind != ResourceKind::Window {
                return None;
            }

            if entry.label.as_deref() != Some(WINDOW_RESOURCE_LABEL) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<Mutex<Win32WindowBinding>>>())
                .map(Arc::clone)
        })
        .flatten();

    resolved.ok_or_else(|| {
        core::not_found(
            operation,
            format!("window handle {} was not found", window.0.0),
        )
    })
}

/// Resolve one monitor-event binding payload from one opened monitor-event handle.
pub(super) fn resolve_monitor_event_binding(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MonitorEventBinding>> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Display {
                return None;
            }

            if entry.label.as_deref() != Some(DISPLAY_EVENT_RESOURCE_LABEL) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<MonitorEventBinding>>())
                .map(Arc::clone)
        })
        .flatten();

    resolved.ok_or_else(|| {
        core::not_found(
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
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Window {
                return None;
            }

            if entry.label.as_deref() != Some(WINDOW_EVENT_RESOURCE_LABEL) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<WindowEventBinding>>())
                .map(Arc::clone)
        })
        .flatten();

    resolved.ok_or_else(|| {
        core::not_found(
            operation,
            format!("window event handle {} was not found", handle.0.0),
        )
    })
}

/// Build one resource entry for one opened window binding.
pub(super) fn window_resource_entry(
    hwnd: HWND,
    binding: Arc<Mutex<Win32WindowBinding>>,
) -> ResourceEntry {
    ResourceEntry::new(ResourceKind::Window)
        .with_label(WINDOW_RESOURCE_LABEL)
        .with_handle(hwnd as *mut c_void)
        .with_payload(binding)
        .with_finalizer(Win32WindowFinalizer { hwnd })
}
