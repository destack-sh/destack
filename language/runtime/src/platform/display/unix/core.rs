use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
    DisplayBackendSelectionPolicy,
};
use crate::platform::{PlatformError, display as display_platform};
use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Wayland, DisplayBackend::X11];
#[cfg(target_os = "macos")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::AppKit];
#[cfg(target_os = "android")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Android];
#[cfg(target_os = "ios")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::UIKit];
#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "android",
        target_os = "ios"
    ))
))]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[];

/// Return unix display backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [DisplayBackend] {
    UNIX_BACKEND_PRIORITY
}

/// Return one backend name for diagnostics and descriptors.
pub(crate) fn backend_name(backend: DisplayBackend) -> &'static str {
    match backend {
        DisplayBackend::Wayland => "wayland",
        DisplayBackend::X11 => "x11",
        DisplayBackend::AppKit => "appkit",
        DisplayBackend::UIKit => "uikit",
        DisplayBackend::Android => "android",
        DisplayBackend::Win32 => "win32",
        DisplayBackend::Null => "null",
        DisplayBackend::Auto => "auto",
    }
}

/// Return whether one backend is valid for the active unix target.
pub(crate) fn backend_supported(backend: DisplayBackend) -> bool {
    #[cfg(target_os = "linux")]
    if matches!(backend, DisplayBackend::Wayland | DisplayBackend::X11) {
        return true;
    }

    #[cfg(target_os = "macos")]
    if backend == DisplayBackend::AppKit {
        return true;
    }

    #[cfg(target_os = "android")]
    if backend == DisplayBackend::Android {
        return true;
    }

    #[cfg(target_os = "ios")]
    if backend == DisplayBackend::UIKit {
        return true;
    }

    let _ = backend;
    false
}

/// Return whether one backend is currently available.
pub(crate) fn backend_available(backend: DisplayBackend) -> bool {
    if !backend_supported(backend) {
        return false;
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::Wayland {
        return std::env::var_os("WAYLAND_DISPLAY").is_some();
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::X11 {
        return std::env::var_os("DISPLAY").is_some();
    }

    true
}

/// Return one backend capability mask for one unix backend.
pub(crate) fn backend_capabilities(backend: DisplayBackend) -> DisplayBackendCapabilityFlags {
    if !backend_supported(backend) {
        return DisplayBackendCapabilityFlags(0);
    }

    DisplayBackendCapabilityFlags(
        display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
            | display_platform::DISPLAY_BACKEND_CAP_EXCLUSIVE_FULLSCREEN.0
            | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0
            | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0,
    )
}

/// Build one not-supported error for one unix display backend operation.
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

/// List unix display backend descriptors.
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
    for backend in preferred_host_backends().iter().copied() {
        if backend_supported(backend) {
            return Ok(backend);
        }
    }

    Err(RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: no unix display backend is supported on this target",
    )))
    .boxed())
}

/// Resolve one auto-selected backend for one operation.
fn resolve_auto_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    for backend in preferred_host_backends().iter().copied() {
        if backend_available(backend) {
            return Ok(backend);
        }
    }

    for backend in preferred_host_backends().iter().copied() {
        if backend_supported(backend) {
            return Ok(backend);
        }
    }

    Err(RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: no unix display backend is available on this target",
    )))
    .boxed())
}
