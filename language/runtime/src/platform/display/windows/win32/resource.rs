use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetWindowThreadProcessId, IsWindow, PostMessageW, WM_CLOSE,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{
    ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, resolve_payload,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::constants::*;
use super::core;
use super::event::{MonitorEventBinding, WindowEventBinding};
use super::model::{Win32DisplayBinding, Win32WindowBinding};

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
                    let _ = PostMessageW(self.hwnd, WM_CLOSE, core::WINDOW_CLOSE_FORCE_WPARAM, 0);
                }
            }
        }
    }
}

/// Insert one monitor resource for one monitor identifier.
pub(super) fn open_display_handle(
    binding: &BindingCallContext,
    id: String,
) -> resource::DisplayHandle {
    let entry = ResourceEntry::new(ResourceKind::Display)
        .with_label(DISPLAY_RESOURCE_LABEL)
        .with_payload(Win32DisplayBinding { id });
    let resource_id = binding
        .agent()
        .resources
        .insert(entry, Some(binding.engine()));
    resource::DisplayHandle(resource_id)
}

/// Resolve one monitor identifier from one opened display handle.
pub(super) fn resolve_display_id(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<String> {
    let resolved_binding = resolve_payload::<Win32DisplayBinding>(
        binding,
        handle.0,
        ResourceKind::Display,
        Some(DISPLAY_RESOURCE_LABEL),
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display handle {} was not found", handle.0.0),
        )
    })?;

    Ok(resolved_binding.id)
}

/// Resolve one window binding payload from one opened window handle.
pub(super) fn resolve_window_binding(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<Win32WindowBinding>>> {
    resolve_payload::<Arc<Mutex<Win32WindowBinding>>>(
        binding,
        window.0,
        ResourceKind::Window,
        Some(WINDOW_RESOURCE_LABEL),
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("window handle {} was not found", window.0.0),
        )
    })
}

/// Validate that one window handle resolves to one win32 window binding.
pub(crate) fn ensure_window_binding_exists(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    resolve_window_binding(context, window, operation)?;

    Ok(())
}

/// Resolve one monitor-event binding payload from one opened monitor-event handle.
pub(super) fn resolve_monitor_event_binding(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MonitorEventBinding>> {
    resolve_payload::<Arc<MonitorEventBinding>>(
        binding,
        handle.0,
        ResourceKind::Display,
        Some(DISPLAY_EVENT_RESOURCE_LABEL),
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
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowEventBinding>> {
    resolve_payload::<Arc<WindowEventBinding>>(
        binding,
        handle.0,
        ResourceKind::Window,
        Some(WINDOW_EVENT_RESOURCE_LABEL),
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
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
