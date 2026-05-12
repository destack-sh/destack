use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform;
use crate::platform::PlatformError;
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayMonitorEventKindMask, WindowEventKindMask,
};
use crate::platform::resource::{DisplayHandle, WindowHandle};
use crate::runtime::BindingCallContext;

pub(crate) use crate::platform::display::unix::appkit::constants::*;

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
    platform::core::io_not_found(
        operation,
        format!("display handle {} was not found", handle.0.local_id),
    )
}

/// Build one missing window-handle error.
pub(crate) fn window_not_found(operation: &'static str, handle: WindowHandle) -> Box<RuntimeError> {
    platform::core::io_not_found(
        operation,
        format!("window handle {} was not found", handle.0.local_id),
    )
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

    Err(platform::core::invalid_argument(
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

    Err(platform::core::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Build one busy error for queued-event overflow with `Error` policy.
pub(crate) fn overflow_error(operation: &'static str) -> Box<RuntimeError> {
    platform::core::io_busy(
        operation,
        "event queue overflowed while overflow policy is error",
    )
}

/// Return backend descriptor support and capability flags for AppKit.
pub(crate) fn backend_descriptor_state(
    _binding: &BindingCallContext,
) -> (BackendSupport, DisplayBackendCapabilityFlags) {
    let capability_flags = platform::display::DISPLAY_BACKEND_CAP_WINDOW.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | platform::display::DISPLAY_BACKEND_CAP_MONITOR.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | platform::display::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | platform::display::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0
        | platform::display::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0
        | platform::display::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | platform::display::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
        | platform::display::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0
        | platform::display::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
        | platform::display::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
        | platform::display::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0
        | platform::display::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
        | platform::display::DISPLAY_BACKEND_CAP_BEGIN_FRAME_STREAM.0
        | platform::display::DISPLAY_BACKEND_CAP_SAFE_AREA.0
        | platform::display::DISPLAY_BACKEND_CAP_THEME.0
        | platform::display::DISPLAY_BACKEND_CAP_OCCLUSION.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
        | platform::display::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0;

    (
        if backend_available() {
            BackendSupport::Available
        } else {
            BackendSupport::HostUnavailable
        },
        DisplayBackendCapabilityFlags(capability_flags),
    )
}
