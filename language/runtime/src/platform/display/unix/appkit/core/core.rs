use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::{
    self as display_platform, DisplayBackend, DisplayBackendCapabilityFlags,
    DisplayMonitorEventKindMask, WindowEventKindMask,
};
use crate::platform::resource::{DisplayHandle, WindowHandle};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

pub(crate) use super::super::constants::*;

/// Return the backend for the AppKit backend implementation.
pub(crate) fn selected_backend() -> DisplayBackend {
    DisplayBackend::AppKit
}

/// Return one backend label for AppKit diagnostics and stable identifiers.
pub(crate) fn selected_backend_name() -> &'static str {
    "appkit"
}

/// Return whether the current call can safely drive AppKit.
pub(crate) fn backend_available() -> bool {
    true
}

/// Build one mapped AppKit I/O error payload.
pub(crate) fn io_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::Io),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one missing display-handle error.
pub(crate) fn display_not_found(
    operation: &'static str,
    handle: DisplayHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("display handle {} was not found", handle.0.0),
    )
}

/// Build one missing window-handle error.
pub(crate) fn window_not_found(operation: &'static str, handle: WindowHandle) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("window handle {} was not found", handle.0.0),
    )
}

/// Return the configured default display event queue capacity.
pub(crate) fn default_event_queue_capacity(binding: &BindingCallContext) -> usize {
    let _binding = binding;
    DEFAULT_EVENT_QUEUE_CAPACITY
}

/// Resolve queue capacity for one event stream open request.
pub(crate) fn resolved_queue_capacity(binding: &BindingCallContext, value: u32) -> usize {
    // fall back to the runtime default when the caller leaves capacity unset
    if value == 0 {
        return default_event_queue_capacity(binding);
    }

    core_platform::u32_to_usize(value)
}

/// Return the configured window-event wait slice duration in nanoseconds.
pub(crate) fn window_event_wait_slice_ns(binding: &BindingCallContext) -> u64 {
    let _binding = binding;
    DEFAULT_EVENT_WAIT_SLICE_NS
}

/// Record one AppKit callback warning for one swallowed backend error.
pub(crate) fn warn_callback_error(
    runtime_state: &super::runtime::AppKitRuntimeState,
    operation: &'static str,
    error: &RuntimeError,
) {
    runtime_state
        .diagnostics
        .warn("display", operation, error.to_string(), None);
}

/// Resolve one monitor-event kind mask.
pub(crate) fn monitor_kind_mask(value: Option<DisplayMonitorEventKindMask>) -> u32 {
    value.map_or(DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Resolve one window-event kind mask.
pub(crate) fn window_kind_mask(value: Option<WindowEventKindMask>) -> u64 {
    value.map_or(WINDOW_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Validate one monitor-event filter bit-mask payload.
pub(crate) fn validate_monitor_event_kind_mask(
    kind_mask: u32,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !DISPLAY_MONITOR_EVENT_KIND_MASK_ALL;

    // return early when all requested bits are supported
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported monitor event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Validate one window-event filter bit-mask payload.
pub(crate) fn validate_window_event_kind_mask(
    kind_mask: u64,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !WINDOW_EVENT_KIND_MASK_ALL;

    // return early when all requested bits are supported
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Build one busy error for queued-event overflow with `Error` policy.
pub(crate) fn overflow_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_busy(
        operation,
        "event queue overflowed while overflow policy is error",
    )
}

/// Drain stale weak entries and skip one identity from one weak registry.
pub(crate) fn retain_live_without_identity<T>(
    registry: &mut Vec<std::sync::Weak<T>>,
    identity: usize,
) {
    registry.retain(|weak| {
        let Some(strong) = weak.upgrade() else {
            return false;
        };

        std::sync::Arc::as_ptr(&strong) as usize != identity
    });
}

/// Return backend descriptor availability and capability flags for AppKit.
pub(crate) fn backend_descriptor_state(
    _binding: &BindingCallContext,
) -> (bool, DisplayBackendCapabilityFlags) {
    let capability_flags = display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0
        | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
        | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
        | display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0
        | display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_REFRESH_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_SAFE_AREA.0
        | display_platform::DISPLAY_BACKEND_CAP_THEME.0
        | display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0;

    (
        backend_available(),
        DisplayBackendCapabilityFlags(capability_flags),
    )
}
