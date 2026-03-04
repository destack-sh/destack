use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
    DisplayBackendSelectionPolicy,
};
use crate::platform::{PlatformError, display as display_platform};
use crate::runtime::BindingCallContext;

const WINDOWS_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Win32];

/// Return windows display backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [DisplayBackend] {
    WINDOWS_BACKEND_PRIORITY
}

/// Return one backend name for diagnostics and descriptors.
pub(crate) fn backend_name(backend: DisplayBackend) -> &'static str {
    // map backend enum to stable diagnostics name
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
    // return empty mask for unsupported backends
    if !backend_supported(backend) {
        return DisplayBackendCapabilityFlags(0);
    }

    // report win32 capability lanes supported by this backend
    DisplayBackendCapabilityFlags(
        display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR_HDR_CONTROL.0
            | display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0
            | display_platform::DISPLAY_BACKEND_CAP_EXCLUSIVE_FULLSCREEN.0
            | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0
            | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
            | display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0
            | display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
            | display_platform::DISPLAY_BACKEND_CAP_REFRESH_REQUEST.0
            | display_platform::DISPLAY_BACKEND_CAP_THEME.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0,
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
    // allocate descriptor list for preferred backend order
    let mut descriptors = Vec::with_capacity(preferred_host_backends().len());

    // build one descriptor per preferred backend
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
