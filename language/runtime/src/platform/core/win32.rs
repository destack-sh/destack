use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Networking::WinSock::WSAGetLastError;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, PlatformErrorCode};

/// Return the last Win32 error code.
pub(crate) fn last_error_code() -> i32 {
    unsafe { GetLastError() as i32 }
}

/// Return the last Winsock error code.
pub(crate) fn last_wsa_error_code() -> i32 {
    unsafe { WSAGetLastError() }
}

/// Format a simple Win32 error message.
pub(crate) fn error_message(syscall: &str, code: i32) -> String {
    format!("{syscall} failed: {code}")
}

/// Build an I/O runtime error from the last Win32 error.
pub(crate) fn io_error(syscall: &str) -> Box<RuntimeError> {
    let code = last_error_code();
    io_error_with_code(syscall, code)
}

/// Build an I/O runtime error from an explicit Win32 error code.
pub(crate) fn io_error_with_code(syscall: &str, code: i32) -> Box<RuntimeError> {
    let message = error_message(syscall, code);
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some(syscall.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Build an I/O runtime error from an explicit Win32 code and mapped platform code.
pub(crate) fn io_error_with_platform_code(
    syscall: &str,
    win32_code: i32,
    platform_code: PlatformErrorCode,
) -> Box<RuntimeError> {
    let message = error_message(syscall, win32_code);
    RuntimeError::from(PlatformError::io_with(
        Some(platform_code),
        None,
        Some(win32_code),
        Some(syscall.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Build a network runtime error from the last Winsock error.
#[allow(dead_code)]
pub(crate) fn net_error(syscall: &str) -> Box<RuntimeError> {
    let code = last_wsa_error_code();
    net_error_with_code(syscall, code)
}

/// Build a network runtime error from an explicit Winsock error code.
pub(crate) fn net_error_with_code(syscall: &str, code: i32) -> Box<RuntimeError> {
    let message = error_message(syscall, code);
    RuntimeError::from(PlatformError::net_with(
        None,
        None,
        Some(code),
        Some(syscall.to_string()),
        None,
        None,
        message,
    ))
    .boxed()
}

/// Convert a utf-8 byte slice into a nul-terminated wide string.
pub(crate) fn wide_from_utf8(label: &str, bytes: &[u8]) -> RuntimeResult<Vec<u16>> {
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains nul byte",
        ))
        .boxed());
    }

    let value = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains invalid utf8",
        ))
        .boxed()
    })?;
    wide_from_str(label, value)
}

/// Convert a utf-8 byte slice into a String.
pub(crate) fn string_from_utf8(label: &str, bytes: &[u8]) -> RuntimeResult<String> {
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains nul byte",
        ))
        .boxed());
    }

    let value = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains invalid utf8",
        ))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Convert a utf-8 byte slice into a PathBuf.
pub(crate) fn pathbuf_from_utf8(label: &str, bytes: &[u8]) -> RuntimeResult<PathBuf> {
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains nul byte",
        ))
        .boxed());
    }

    let value = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains invalid utf8",
        ))
        .boxed()
    })?;

    Ok(PathBuf::from(value))
}

/// Convert a utf-16 slice into a nul-terminated wide string.
pub(crate) fn wide_from_utf16(label: &str, units: &[u16]) -> RuntimeResult<Vec<u16>> {
    if units.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains nul code unit",
        ))
        .boxed());
    }

    let mut wide = units.to_vec();
    wide.push(0);
    Ok(wide)
}

/// Convert a string into a nul-terminated wide string.
pub(crate) fn wide_from_str(label: &str, value: &str) -> RuntimeResult<Vec<u16>> {
    if value.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value contains nul byte",
        ))
        .boxed());
    }

    let mut wide: Vec<u16> = value.encode_utf16().collect();
    wide.push(0);
    Ok(wide)
}

/// Convert a wide string buffer into a String.
pub(crate) fn string_from_wide(label: &str, buffer: &[u16]) -> RuntimeResult<String> {
    String::from_utf16(buffer).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            format!("{label} contains invalid utf16"),
        ))
        .boxed()
    })
}

/// Convert a UTF-16 slice into a PathBuf.
pub(crate) fn pathbuf_from_utf16(label: &str, units: &[u16]) -> RuntimeResult<PathBuf> {
    if units.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            format!("{label} contains nul code unit"),
        ))
        .boxed());
    }

    Ok(PathBuf::from(OsString::from_wide(units)))
}
