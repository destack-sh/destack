use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform, display as display_platform};
use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER,
};

/// Default queue capacity for monitor and window event streams.
pub(super) const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
/// Default wait slice for window event blocking reads.
pub(super) const DEFAULT_WINDOW_EVENT_WAIT_SLICE_NS: u64 = 10_000_000;
/// Window message `wparam` lane used for force-close paths.
pub(super) const WINDOW_CLOSE_FORCE_WPARAM: usize = 1;
/// Display metric mask bit for bounds updates.
pub(super) const DISPLAY_CHANGED_MASK_BOUNDS: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_BOUNDS.0;
/// Display metric mask bit for work-area updates.
pub(super) const DISPLAY_CHANGED_MASK_WORKAREA: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_WORK_AREA.0;
/// Display metric mask bit for scale updates.
pub(super) const DISPLAY_CHANGED_MASK_SCALE: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_SCALE_FACTOR.0;
/// Display metric mask bit for orientation updates.
pub(super) const DISPLAY_CHANGED_MASK_ORIENTATION: u32 =
    display_platform::DISPLAY_METRIC_CHANGED_ORIENTATION.0;
/// Monitor-event kind bit for `added`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_ADDED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_ADDED.0;
/// Monitor-event kind bit for `removed`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_REMOVED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_REMOVED.0;
/// Monitor-event kind bit for `primaryChanged`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED.0;
/// Monitor-event kind bit for `descriptorChanged`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED.0;
/// Monitor-event kind bit for `modeChanged`.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED: u32 =
    display_platform::DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED.0;
/// All supported monitor-event kind bits.
pub(super) const DISPLAY_MONITOR_EVENT_KIND_MASK_ALL: u32 = DISPLAY_MONITOR_EVENT_KIND_ADDED
    | DISPLAY_MONITOR_EVENT_KIND_REMOVED
    | DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED
    | DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED
    | DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED;
/// Window-event kind bit for `created`.
pub(super) const WINDOW_EVENT_KIND_CREATED: u64 = display_platform::WINDOW_EVENT_KIND_CREATED.0;
/// Window-event kind bit for `closeRequested`.
pub(super) const WINDOW_EVENT_KIND_CLOSE_REQUESTED: u64 =
    display_platform::WINDOW_EVENT_KIND_CLOSE_REQUESTED.0;
/// Window-event kind bit for `destroyed`.
pub(super) const WINDOW_EVENT_KIND_DESTROYED: u64 = display_platform::WINDOW_EVENT_KIND_DESTROYED.0;
/// Window-event kind bit for `focusChanged`.
pub(super) const WINDOW_EVENT_KIND_FOCUS_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_FOCUS_CHANGED.0;
/// Window-event kind bit for `visibilityChanged`.
pub(super) const WINDOW_EVENT_KIND_VISIBILITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_VISIBILITY_CHANGED.0;
/// Window-event kind bit for `occlusionChanged`.
pub(super) const WINDOW_EVENT_KIND_OCCLUSION_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_OCCLUSION_CHANGED.0;
/// Window-event kind bit for `positionChanged`.
pub(super) const WINDOW_EVENT_KIND_POSITION_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_POSITION_CHANGED.0;
/// Window-event kind bit for `sizeChanged`.
pub(super) const WINDOW_EVENT_KIND_SIZE_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SIZE_CHANGED.0;
/// Window-event kind bit for `scaleFactorChanged`.
pub(super) const WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED.0;
/// Window-event kind bit for `refreshRequested`.
pub(super) const WINDOW_EVENT_KIND_REFRESH_REQUESTED: u64 =
    display_platform::WINDOW_EVENT_KIND_REFRESH_REQUESTED.0;
/// Window-event kind bit for `modeChanged`.
pub(super) const WINDOW_EVENT_KIND_MODE_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MODE_CHANGED.0;
/// Window-event kind bit for `displayChanged`.
pub(super) const WINDOW_EVENT_KIND_DISPLAY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_DISPLAY_CHANGED.0;
/// Window-event kind bit for `themeChanged`.
pub(super) const WINDOW_EVENT_KIND_THEME_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_THEME_CHANGED.0;
/// Window-event kind bit for `chromeChanged`.
pub(super) const WINDOW_EVENT_KIND_CHROME_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_CHROME_CHANGED.0;
/// Window-event kind bit for `taskbarVisibilityChanged`.
pub(super) const WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED.0;
/// Window-event kind bit for `opacityChanged`.
pub(super) const WINDOW_EVENT_KIND_OPACITY_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_OPACITY_CHANGED.0;
/// Window-event kind bit for `parentChanged`.
pub(super) const WINDOW_EVENT_KIND_PARENT_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_PARENT_CHANGED.0;
/// Window-event kind bit for `transientChanged`.
pub(super) const WINDOW_EVENT_KIND_TRANSIENT_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_TRANSIENT_CHANGED.0;
/// Window-event kind bit for `modalChanged`.
pub(super) const WINDOW_EVENT_KIND_MODAL_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MODAL_CHANGED.0;
/// Window-event kind bit for `mousePassthroughChanged`.
pub(super) const WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED.0;
/// Window-event kind bit for `aspectRatioChanged`.
pub(super) const WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED: u64 =
    display_platform::WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED.0;
/// Window-event kind bit for `dropStarted`.
pub(super) const WINDOW_EVENT_KIND_DROP_STARTED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_STARTED.0;
/// Window-event kind bit for `fileHovered`.
pub(super) const WINDOW_EVENT_KIND_FILE_HOVERED: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_HOVERED.0;
/// Window-event kind bit for `dropCancelled`.
pub(super) const WINDOW_EVENT_KIND_DROP_CANCELLED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_CANCELLED.0;
/// Window-event kind bit for `dropCompleted`.
pub(super) const WINDOW_EVENT_KIND_DROP_COMPLETED: u64 =
    display_platform::WINDOW_EVENT_KIND_DROP_COMPLETED.0;
/// Window-event kind bit for `fileHoverLeft`.
pub(super) const WINDOW_EVENT_KIND_FILE_HOVER_LEFT: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_HOVER_LEFT.0;
/// Window-event kind bit for `fileDropped`.
pub(super) const WINDOW_EVENT_KIND_FILE_DROPPED: u64 =
    display_platform::WINDOW_EVENT_KIND_FILE_DROPPED.0;
/// Window-event kind bit for `textDropped`.
pub(super) const WINDOW_EVENT_KIND_TEXT_DROPPED: u64 =
    display_platform::WINDOW_EVENT_KIND_TEXT_DROPPED.0;
/// All supported window-event kind bits.
pub(super) const WINDOW_EVENT_KIND_MASK_ALL: u64 = WINDOW_EVENT_KIND_CREATED
    | WINDOW_EVENT_KIND_CLOSE_REQUESTED
    | WINDOW_EVENT_KIND_DESTROYED
    | WINDOW_EVENT_KIND_FOCUS_CHANGED
    | WINDOW_EVENT_KIND_VISIBILITY_CHANGED
    | WINDOW_EVENT_KIND_OCCLUSION_CHANGED
    | WINDOW_EVENT_KIND_POSITION_CHANGED
    | WINDOW_EVENT_KIND_SIZE_CHANGED
    | WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED
    | WINDOW_EVENT_KIND_REFRESH_REQUESTED
    | WINDOW_EVENT_KIND_MODE_CHANGED
    | WINDOW_EVENT_KIND_DISPLAY_CHANGED
    | WINDOW_EVENT_KIND_THEME_CHANGED
    | WINDOW_EVENT_KIND_CHROME_CHANGED
    | WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED
    | WINDOW_EVENT_KIND_OPACITY_CHANGED
    | WINDOW_EVENT_KIND_PARENT_CHANGED
    | WINDOW_EVENT_KIND_TRANSIENT_CHANGED
    | WINDOW_EVENT_KIND_MODAL_CHANGED
    | WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED
    | WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED
    | WINDOW_EVENT_KIND_DROP_STARTED
    | WINDOW_EVENT_KIND_FILE_HOVERED
    | WINDOW_EVENT_KIND_DROP_CANCELLED
    | WINDOW_EVENT_KIND_DROP_COMPLETED
    | WINDOW_EVENT_KIND_FILE_HOVER_LEFT
    | WINDOW_EVENT_KIND_FILE_DROPPED
    | WINDOW_EVENT_KIND_TEXT_DROPPED;

/// Build one mapped windows I/O error payload.
pub(super) fn io_error_with_code(
    operation: &'static str,
    syscall: &'static str,
    code: u32,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let platform_code = if code == ERROR_INVALID_HANDLE || code == ERROR_INVALID_PARAMETER {
        Some(PlatformErrorCode::IoNotFound)
    } else if code == ERROR_ACCESS_DENIED {
        Some(PlatformErrorCode::IoPermissionDenied)
    } else {
        None
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(code as i32),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {} ({code})", message.into()),
    ))
    .boxed()
}

/// Build one mapped windows I/O error payload from last-error state.
pub(super) fn io_error(
    operation: &'static str,
    syscall: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let code = core_platform::last_error_code() as u32;
    io_error_with_code(operation, syscall, code, message)
}

/// Return the configured default display event queue capacity.
pub(super) fn default_event_queue_capacity(context: &BindingCallContext) -> usize {
    let configured = context
        .runtime()
        .module_options
        .display
        .default_event_queue_capacity;
    core_platform::option_u64_to_usize_or_min(configured, DEFAULT_EVENT_QUEUE_CAPACITY, 1)
}

/// Resolve queue capacity for one event stream open request.
pub(super) fn resolved_queue_capacity(context: &BindingCallContext, value: u32) -> usize {
    if value == 0 {
        return default_event_queue_capacity(context);
    }

    value as usize
}

/// Return the configured window-event wait slice duration in nanoseconds.
pub(super) fn window_event_wait_slice_ns(context: &BindingCallContext) -> u64 {
    let configured = context
        .runtime()
        .module_options
        .display
        .window_event_wait_slice_ns;
    core_platform::option_u64_or_min(configured, DEFAULT_WINDOW_EVENT_WAIT_SLICE_NS, 1)
}

/// Validate one batch-size payload.
pub(super) fn validate_max_events(maxevents: u32, field: &'static str) -> RuntimeResult<usize> {
    if maxevents == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "value must be greater than zero",
        ));
    }

    Ok(maxevents as usize)
}

/// Validate one monitor-event filter bit-mask payload.
pub(super) fn validate_monitor_event_kind_mask(
    kind_mask: u32,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !DISPLAY_MONITOR_EVENT_KIND_MASK_ALL;
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported monitor event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Validate one window-event filter bit-mask payload.
pub(super) fn validate_window_event_kind_mask(
    kind_mask: u64,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !WINDOW_EVENT_KIND_MASK_ALL;
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Convert one fixed wide buffer into one owned utf-8 string.
pub(super) fn utf16_buffer_to_string(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}
