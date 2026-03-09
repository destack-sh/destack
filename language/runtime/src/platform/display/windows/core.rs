use crate::diagnostic::RuntimeResult;
use crate::platform::core::{BackendSupport, aggregate_backend_support, backend_support_error};
use crate::platform::display as display_platform;
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
    DisplayBackendSelectionPolicy,
};
use crate::runtime::BindingCallContext;

const WINDOWS_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Win32];
const DISPLAY_BACKEND_SELECTORS: &[DisplayBackend] = &[
    DisplayBackend::Auto,
    DisplayBackend::Wayland,
    DisplayBackend::X11,
    DisplayBackend::Win32,
    DisplayBackend::AppKit,
    DisplayBackend::UIKit,
    DisplayBackend::Android,
    DisplayBackend::Null,
];

/// Descriptor support and capabilities for one backend row.
#[derive(Clone, Copy)]
struct DisplayBackendDescriptorState {
    /// The host integration support state.
    support: BackendSupport,
    /// The advertised capability mask for this backend row.
    capability_flags: DisplayBackendCapabilityFlags,
}

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

/// Return one cached descriptor state for the win32 backend row.
fn backend_descriptor_state(backend: DisplayBackend) -> DisplayBackendDescriptorState {
    DisplayBackendDescriptorState {
        support: backend_support(backend),
        capability_flags: backend_capabilities(backend),
    }
}

/// Return the first available windows host backend on the current host.
fn active_host_backend() -> Option<DisplayBackend> {
    preferred_host_backends()
        .iter()
        .copied()
        .find(|backend| backend_support(*backend).is_available())
}

/// Return combined support for the default host backend lane.
fn auto_backend_support() -> BackendSupport {
    aggregate_backend_support(
        preferred_host_backends()
            .iter()
            .copied()
            .map(backend_support),
    )
}

/// Return one selector descriptor state.
fn selector_descriptor_state(
    backend: DisplayBackend,
    active_backend: Option<DisplayBackend>,
) -> DisplayBackendDescriptorState {
    // auto mirrors the active host backend capability row
    if backend == DisplayBackend::Auto {
        let capability_flags = active_backend
            .map(backend_capabilities)
            .unwrap_or(DisplayBackendCapabilityFlags(0));

        return DisplayBackendDescriptorState {
            support: auto_backend_support(),
            capability_flags,
        };
    }

    // win32 exposes its host capability row directly
    if backend == DisplayBackend::Win32 {
        return backend_descriptor_state(backend);
    }

    // unsupported selectors expose support only and no host capabilities
    DisplayBackendDescriptorState {
        support: backend_support(backend),
        capability_flags: DisplayBackendCapabilityFlags(0),
    }
}

/// Return one auto-selection priority for one descriptor row.
fn backend_priority(backend: DisplayBackend) -> u16 {
    if backend == DisplayBackend::Auto {
        return u16::MAX;
    }

    preferred_host_backends()
        .iter()
        .position(|candidate| *candidate == backend)
        .map(|index| u16::MAX.saturating_sub(index as u16 + 1))
        .unwrap_or(0)
}

/// Return host backend support for one windows display backend.
pub(crate) fn backend_support(backend: DisplayBackend) -> BackendSupport {
    // auto reflects the combined preferred host lane
    if backend == DisplayBackend::Auto {
        return auto_backend_support();
    }

    // null is not a windows display host backend
    if backend == DisplayBackend::Null {
        return BackendSupport::UnsupportedTarget;
    }

    // win32 is the one windows host integration
    if backend == DisplayBackend::Win32 {
        return BackendSupport::Available;
    }

    BackendSupport::UnsupportedTarget
}

/// Return one backend capability mask for one windows backend.
pub(crate) fn backend_capabilities(backend: DisplayBackend) -> DisplayBackendCapabilityFlags {
    // zero capability flags for unavailable backend rows
    if !backend_support(backend).is_available() {
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
            | display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0
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
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0,
    )
}

/// List windows display backend descriptors.
pub(crate) fn backend_descriptors(binding: &BindingCallContext) -> Vec<DisplayBackendDescriptor> {
    // resolve the active host backend for auto rows
    let active_backend = active_host_backend();
    let mut descriptors = Vec::with_capacity(DISPLAY_BACKEND_SELECTORS.len());

    // emit one stable descriptor row per selector
    for backend in DISPLAY_BACKEND_SELECTORS.iter().copied() {
        let state = selector_descriptor_state(backend, active_backend);

        descriptors.push(DisplayBackendDescriptor {
            backend,
            name: binding.store_string(backend_name(backend)),
            support: state.support,
            priority: backend_priority(backend),
            capability_flags: state.capability_flags,
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

    let support = backend_support(requested);
    if support.is_available() {
        return Ok(requested);
    }

    if selection_policy == DisplayBackendSelectionPolicy::AllowFallback {
        return resolve_auto_backend(operation);
    }

    Err(backend_support_error(
        operation,
        backend_name(requested),
        support,
    ))
}

/// Resolve one default backend for operations without an explicit selector.
pub(crate) fn resolve_default_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    resolve_auto_backend(operation)
}

/// Resolve one auto-selected backend for one operation.
fn resolve_auto_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    // return the first reachable host backend in priority order
    active_host_backend().ok_or_else(|| {
        backend_support_error(
            operation,
            backend_name(DisplayBackend::Auto),
            auto_backend_support(),
        )
    })
}
