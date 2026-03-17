use windows::core::{Error as WinError, GUID, HSTRING};

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;

/// Convert one WinRT string into one owned Rust string.
pub(crate) fn hstring_to_string(value: &HSTRING) -> String {
    value.to_string()
}

/// Convert one WinRT GUID into one lowercase public string.
pub(crate) fn guid_to_string(value: GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        value.data1,
        value.data2,
        value.data3,
        value.data4[0],
        value.data4[1],
        value.data4[2],
        value.data4[3],
        value.data4[4],
        value.data4[5],
        value.data4[6],
        value.data4[7],
    )
}

/// Build one runtime I/O error from one WinRT failure.
pub(crate) fn winrt_io_error(
    operation: &'static str,
    action: &str,
    error: &WinError,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        Some(action.to_string()),
        format!("{action}: {error}"),
    ))
    .boxed()
}
