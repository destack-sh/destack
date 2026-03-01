use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
    DisplayBackendSelectionPolicy,
};
use crate::runtime::BindingCallContext;

const WINDOWS_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Win32];
const DISPLAY_CAP_WINDOW: u64 = 0x1;
const DISPLAY_CAP_MONITOR: u64 = 0x2;
const DISPLAY_CAP_WINDOW_EVENTS: u64 = 0x4;
const DISPLAY_CAP_MONITOR_EVENTS: u64 = 0x8;
const DISPLAY_CAP_EXCLUSIVE_FULLSCREEN: u64 = 0x10;
const DISPLAY_CAP_BORDERLESS_FULLSCREEN: u64 = 0x20;
const DISPLAY_CAP_CURSOR_LOCK: u64 = 0x40;
const DISPLAY_CAP_CURSOR_CONFINE: u64 = 0x80;
const DISPLAY_CAP_VSYNC_WAIT: u64 = 0x100;
const DISPLAY_CAP_TRANSPARENCY: u64 = 0x200;

/// Return windows display backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [DisplayBackend] {
    WINDOWS_BACKEND_PRIORITY
}

/// Return one backend name for diagnostics and descriptors.
pub(crate) fn backend_name(backend: DisplayBackend) -> &'static str {
    match backend {
        DisplayBackend::Win32 => "win32",
        DisplayBackend::Wayland => "wayland",
        DisplayBackend::X11 => "x11",
        DisplayBackend::AppKit => "appkit",
        DisplayBackend::UIKit => "uikit",
        DisplayBackend::Android => "android",
        DisplayBackend::Null => "null",
        DisplayBackend::Auto => "auto",
    }
}

/// Return whether one backend is valid for the active windows target.
pub(crate) fn backend_supported(backend: DisplayBackend) -> bool {
    backend == DisplayBackend::Win32
}

/// Return whether one backend is currently available.
pub(crate) fn backend_available(backend: DisplayBackend) -> bool {
    backend_supported(backend)
}

/// Return one backend capability mask for one windows backend.
pub(crate) fn backend_capabilities(backend: DisplayBackend) -> DisplayBackendCapabilityFlags {
    if !backend_supported(backend) {
        return DisplayBackendCapabilityFlags(0);
    }

    DisplayBackendCapabilityFlags(
        DISPLAY_CAP_WINDOW
            | DISPLAY_CAP_MONITOR
            | DISPLAY_CAP_WINDOW_EVENTS
            | DISPLAY_CAP_MONITOR_EVENTS
            | DISPLAY_CAP_EXCLUSIVE_FULLSCREEN
            | DISPLAY_CAP_BORDERLESS_FULLSCREEN
            | DISPLAY_CAP_CURSOR_LOCK
            | DISPLAY_CAP_CURSOR_CONFINE
            | DISPLAY_CAP_VSYNC_WAIT
            | DISPLAY_CAP_TRANSPARENCY,
    )
}

/// Build one not-supported error for one windows display backend operation.
pub(crate) fn backend_not_supported(
    operation: &'static str,
    backend: DisplayBackend,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: backend {} is not implemented",
        backend_name(backend)
    )))
    .boxed()
}

/// List windows display backend descriptors.
pub(crate) fn backend_descriptors(context: &BindingCallContext) -> Vec<DisplayBackendDescriptor> {
    let mut descriptors = Vec::with_capacity(preferred_host_backends().len());

    for (index, backend) in preferred_host_backends().iter().copied().enumerate() {
        let priority = u16::MAX.saturating_sub(index as u16);
        descriptors.push(DisplayBackendDescriptor {
            backend,
            name: context.store_string(backend_name(backend)),
            available: backend_available(backend),
            priority,
            capability_flags: backend_capabilities(backend),
        });
    }

    descriptors
}

/// Resolve one backend selection from options for one operation.
pub(crate) fn resolve_backend(
    requested: DisplayBackend,
    selection_policy: DisplayBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<DisplayBackend> {
    if requested == DisplayBackend::Auto {
        return resolve_auto_backend(operation);
    }

    if backend_available(requested) {
        return Ok(requested);
    }

    if selection_policy == DisplayBackendSelectionPolicy::AllowFallback {
        return resolve_auto_backend(operation);
    }

    Err(backend_not_supported(operation, requested))
}

/// Resolve one default backend for operations without an explicit selector.
pub(crate) fn resolve_default_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    preferred_host_backends().first().copied().ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(format!(
            "{operation}: no windows display backend is supported on this target",
        )))
        .boxed()
    })
}

/// Resolve one auto-selected backend for one operation.
fn resolve_auto_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    resolve_default_backend(operation)
}
