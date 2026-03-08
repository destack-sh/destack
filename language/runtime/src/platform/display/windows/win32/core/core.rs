use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER,
};

pub(crate) use crate::platform::display::windows::win32::constants::*;

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

/// Validate one batch-size payload.
pub(crate) fn validate_max_events(maxevents: u32, field: &'static str) -> RuntimeResult<usize> {
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

/// Convert one fixed wide buffer into one owned utf-8 string.
pub(crate) fn utf16_buffer_to_string(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}
