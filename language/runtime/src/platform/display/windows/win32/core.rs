use std::time::Instant;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER,
};

/// Default queue capacity for monitor and window event streams.
pub(super) const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
/// Default wait slice for window event blocking reads.
pub(super) const DEFAULT_WINDOW_EVENT_WAIT_SLICE_NS: u64 = 10_000_000;
/// Nominal vsync interval used by the fallback vsync wait lane.
pub(super) const FALLBACK_VSYNC_INTERVAL_NS: u64 = 16_666_667;
/// Display metric mask bit for bounds updates.
pub(super) const DISPLAY_CHANGED_MASK_BOUNDS: u32 = 0x4;
/// Display metric mask bit for work-area updates.
pub(super) const DISPLAY_CHANGED_MASK_WORKAREA: u32 = 0x8;
/// Display metric mask bit for scale updates.
pub(super) const DISPLAY_CHANGED_MASK_SCALE: u32 = 0x10;
/// Display metric mask bit for orientation updates.
pub(super) const DISPLAY_CHANGED_MASK_ORIENTATION: u32 = 0x20;

/// Shared monotonic epoch for display timestamps.
static TIMESTAMP_EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Return one monotonic timestamp suitable for event metadata payloads.
pub(super) fn now_timestamp_ns() -> u64 {
    let epoch = TIMESTAMP_EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

/// Build one invalid-argument error payload.
pub(super) fn invalid_argument(
    field: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed()
}

/// Build one not-found runtime error.
pub(super) fn not_found(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioWouldBlock runtime error.
pub(super) fn io_would_block(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioBusy runtime error.
pub(super) fn io_busy(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoBusy),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

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

/// Validate one output pointer argument.
pub(super) fn ensure_out<T>(out: *mut T, field: &'static str) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(field)).boxed());
    }

    Ok(())
}

/// Return the configured default display event queue capacity.
pub(super) fn default_event_queue_capacity(context: &BindingCallContext) -> usize {
    let configured = context
        .runtime()
        .module_options
        .display
        .default_event_queue_capacity;
    let configured = configured.and_then(|value| usize::try_from(value).ok());
    let configured = configured.unwrap_or(DEFAULT_EVENT_QUEUE_CAPACITY);

    configured.max(1)
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
    configured
        .unwrap_or(DEFAULT_WINDOW_EVENT_WAIT_SLICE_NS)
        .max(1)
}

/// Return the configured fallback vsync interval in nanoseconds.
pub(super) fn fallback_vsync_interval_ns(context: &BindingCallContext) -> u64 {
    let configured = context
        .runtime()
        .module_options
        .display
        .fallback_vsync_interval_ns;
    configured.unwrap_or(FALLBACK_VSYNC_INTERVAL_NS).max(1)
}

/// Validate one batch-size payload.
pub(super) fn validate_max_events(maxevents: u32, field: &'static str) -> RuntimeResult<usize> {
    if maxevents == 0 {
        return Err(invalid_argument(field, "value must be greater than zero"));
    }

    Ok(maxevents as usize)
}

/// Convert one fixed wide buffer into one owned utf-8 string.
pub(super) fn utf16_buffer_to_string(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

/// Convert one utf-8 string into one nul-terminated utf-16 buffer.
pub(super) fn wide_with_nul(field: &'static str, value: &str) -> RuntimeResult<Vec<u16>> {
    core_platform::wide_from_str(field, value)
}
