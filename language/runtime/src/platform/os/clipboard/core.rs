use crate::diagnostic::RuntimeResult;
use crate::platform::core::{invalid_argument, not_supported};
use crate::platform::os::ClipboardBinaryFormat;

use crate::platform::os::clipboard::backend;

/// Clipboard read-text binding name.
pub(super) const CLIPBOARD_READ_TEXT_OPERATION: &str = "destack.os.clipboard.readText";
/// Clipboard write-text binding name.
pub(super) const CLIPBOARD_WRITE_TEXT_OPERATION: &str = "destack.os.clipboard.writeText";
/// Clipboard read-bytes binding name.
pub(super) const CLIPBOARD_READ_BYTES_OPERATION: &str = "destack.os.clipboard.readBytes";
/// Clipboard write-bytes binding name.
pub(super) const CLIPBOARD_WRITE_BYTES_OPERATION: &str = "destack.os.clipboard.writeBytes";
/// Clipboard has-text binding name.
#[cfg(all(not(windows), not(target_os = "macos")))]
pub(super) const CLIPBOARD_HAS_TEXT_OPERATION: &str = "destack.os.clipboard.hasText";
/// Clipboard sequence binding name.
#[cfg(not(windows))]
pub(super) const CLIPBOARD_SEQUENCE_OPERATION: &str = "destack.os.clipboard.sequence";
/// Clipboard clear binding name.
#[cfg(not(target_os = "macos"))]
pub(super) const CLIPBOARD_CLEAR_OPERATION: &str = "destack.os.clipboard.clear";

/// Query whether one text payload exists.
pub(crate) fn has_text() -> RuntimeResult<bool> {
    backend::has_text()
}

/// Read one text payload.
pub(crate) fn read_text() -> RuntimeResult<String> {
    backend::read_text()
}

/// Write one text payload.
pub(crate) fn write_text(text: &str) -> RuntimeResult<()> {
    if text.contains('\0') {
        return Err(invalid_argument(
            "text",
            "clipboard text must not contain nul bytes",
        ));
    }

    backend::write_text(text)
}

/// Read one clipboard byte payload.
pub(crate) fn read_bytes(format: ClipboardBinaryFormat) -> RuntimeResult<Vec<u8>> {
    match format {
        ClipboardBinaryFormat::TextUtf8 => Ok(read_text()?.into_bytes()),
        ClipboardBinaryFormat::Html => backend::read_html_bytes(),
        ClipboardBinaryFormat::Binary => Err(not_supported(CLIPBOARD_READ_BYTES_OPERATION)),
    }
}

/// Write one clipboard byte payload.
pub(crate) fn write_bytes(format: ClipboardBinaryFormat, bytes: &[u8]) -> RuntimeResult<()> {
    match format {
        ClipboardBinaryFormat::TextUtf8 => {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                invalid_argument("bytes", "clipboard text bytes must be valid utf8")
            })?;

            write_text(text)
        }
        ClipboardBinaryFormat::Html => {
            let html = std::str::from_utf8(bytes).map_err(|_| {
                invalid_argument("bytes", "clipboard html bytes must be valid utf8")
            })?;

            if html.contains('\0') {
                return Err(invalid_argument(
                    "bytes",
                    "clipboard html bytes must not contain nul bytes",
                ));
            }

            backend::write_html_bytes(bytes)
        }
        ClipboardBinaryFormat::Binary => Err(not_supported(CLIPBOARD_WRITE_BYTES_OPERATION)),
    }
}

/// Read one monotonic clipboard sequence.
pub(crate) fn sequence() -> RuntimeResult<u64> {
    backend::sequence()
}

/// Clear the current clipboard payload.
pub(crate) fn clear() -> RuntimeResult<()> {
    backend::clear()
}
