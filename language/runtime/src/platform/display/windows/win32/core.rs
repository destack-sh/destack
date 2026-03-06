use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER,
};

pub(crate) use super::constants::*;

/// Build one mapped windows I/O error payload.
pub(crate) fn io_error_with_code(
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
pub(crate) fn io_error(
    operation: &'static str,
    syscall: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let code = core_platform::last_error_code() as u32;
    io_error_with_code(operation, syscall, code, message)
}

/// Return the configured default display event queue capacity.
pub(crate) fn default_event_queue_capacity(binding: &BindingCallContext) -> usize {
    let configured = binding.agent().options.display.default_event_queue_capacity;
    core_platform::option_u64_to_usize_or_min(configured, DEFAULT_EVENT_QUEUE_CAPACITY, 1)
}

/// Resolve queue capacity for one event stream open request.
pub(crate) fn resolved_queue_capacity(binding: &BindingCallContext, value: u32) -> usize {
    // evaluate this condition
    if value == 0 {
        return default_event_queue_capacity(binding);
    }

    core_platform::u32_to_usize(value)
}

/// Return the configured window-event wait slice duration in nanoseconds.
pub(crate) fn window_event_wait_slice_ns(binding: &BindingCallContext) -> u64 {
    let configured = binding.agent().options.display.window_event_wait_slice_ns;
    core_platform::option_u64_or_min(configured, DEFAULT_WINDOW_EVENT_WAIT_SLICE_NS, 1)
}

/// Validate one batch-size payload.
pub(crate) fn validate_max_events(maxevents: u32, field: &'static str) -> RuntimeResult<usize> {
    // evaluate this condition
    if maxevents == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "value must be greater than zero",
        ));
    }

    Ok(core_platform::u32_to_usize(maxevents))
}

/// Validate one monitor-event filter bit-mask payload.
pub(crate) fn validate_monitor_event_kind_mask(
    kind_mask: u32,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !DISPLAY_MONITOR_EVENT_KIND_MASK_ALL;
    // evaluate this condition
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
    // evaluate this condition
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Convert one fixed wide buffer into one owned utf-8 string.
pub(crate) fn utf16_buffer_to_string(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}
