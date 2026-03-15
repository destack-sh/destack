use crate::diagnostic::RuntimeResult;
use crate::platform::core::not_supported;

use crate::platform::os::clipboard::core::{
    CLIPBOARD_CLEAR_OPERATION, CLIPBOARD_HAS_TEXT_OPERATION, CLIPBOARD_READ_BYTES_OPERATION,
    CLIPBOARD_READ_TEXT_OPERATION, CLIPBOARD_SEQUENCE_OPERATION, CLIPBOARD_WRITE_BYTES_OPERATION,
    CLIPBOARD_WRITE_TEXT_OPERATION,
};

/// Query whether one text payload exists on unsupported hosts.
pub(crate) fn has_text() -> RuntimeResult<bool> {
    Err(not_supported(CLIPBOARD_HAS_TEXT_OPERATION))
}

/// Read one text payload on unsupported hosts.
pub(crate) fn read_text() -> RuntimeResult<String> {
    Err(not_supported(CLIPBOARD_READ_TEXT_OPERATION))
}

/// Write one text payload on unsupported hosts.
pub(crate) fn write_text(_text: &str) -> RuntimeResult<()> {
    Err(not_supported(CLIPBOARD_WRITE_TEXT_OPERATION))
}

/// Read one HTML payload on unsupported hosts.
pub(crate) fn read_html_bytes() -> RuntimeResult<Vec<u8>> {
    Err(not_supported(CLIPBOARD_READ_BYTES_OPERATION))
}

/// Write one HTML payload on unsupported hosts.
pub(crate) fn write_html_bytes(_bytes: &[u8]) -> RuntimeResult<()> {
    Err(not_supported(CLIPBOARD_WRITE_BYTES_OPERATION))
}

/// Read one monotonic clipboard sequence on unsupported hosts.
pub(crate) fn sequence() -> RuntimeResult<u64> {
    Err(not_supported(CLIPBOARD_SEQUENCE_OPERATION))
}

/// Clear clipboard contents on unsupported hosts.
pub(crate) fn clear() -> RuntimeResult<()> {
    Err(not_supported(CLIPBOARD_CLEAR_OPERATION))
}
