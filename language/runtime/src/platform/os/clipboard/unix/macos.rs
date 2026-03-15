use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};
use objc2_foundation::NSString;

use crate::diagnostic::RuntimeResult;
use crate::host::apple::execution::call_process_main_context_if_needed;
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::os::clipboard::core::{
    CLIPBOARD_READ_BYTES_OPERATION, CLIPBOARD_READ_TEXT_OPERATION, CLIPBOARD_SEQUENCE_OPERATION,
    CLIPBOARD_WRITE_BYTES_OPERATION, CLIPBOARD_WRITE_TEXT_OPERATION,
};

fn with_general_pasteboard<R: Send>(
    callback: impl FnOnce(&NSPasteboard) -> RuntimeResult<R> + Send,
) -> RuntimeResult<R> {
    call_process_main_context_if_needed(|| {
        let pasteboard = NSPasteboard::generalPasteboard();

        callback(&pasteboard)
    })
}

/// Query whether one text payload exists on the macOS clipboard backend.
pub(crate) fn has_text() -> RuntimeResult<bool> {
    with_general_pasteboard(|pasteboard| {
        Ok(pasteboard
            .stringForType(unsafe { NSPasteboardTypeString })
            .is_some())
    })
}

/// Read one text payload from the macOS clipboard backend.
pub(crate) fn read_text() -> RuntimeResult<String> {
    with_general_pasteboard(|pasteboard| {
        let value = pasteboard
            .stringForType(unsafe { NSPasteboardTypeString })
            .ok_or_else(|| {
                io_not_found(
                    CLIPBOARD_READ_TEXT_OPERATION,
                    "clipboard text was not found",
                )
            })?;

        Ok(value.to_string())
    })
}

/// Write one text payload through the macOS clipboard backend.
pub(crate) fn write_text(text: &str) -> RuntimeResult<()> {
    with_general_pasteboard(|pasteboard| {
        let text = NSString::from_str(text);

        // clear stale formats before publishing the new text payload
        pasteboard.clearContents();

        if pasteboard.setString_forType(&text, unsafe { NSPasteboardTypeString }) {
            return Ok(());
        }

        Err(io_operation_error(
            CLIPBOARD_WRITE_TEXT_OPERATION,
            None,
            "pasteboard rejected text payload",
        ))
    })
}

/// Read one HTML payload from the macOS clipboard backend.
pub(crate) fn read_html_bytes() -> RuntimeResult<Vec<u8>> {
    with_general_pasteboard(|pasteboard| {
        let value = pasteboard
            .stringForType(unsafe { NSPasteboardTypeHTML })
            .ok_or_else(|| {
                io_not_found(
                    CLIPBOARD_READ_BYTES_OPERATION,
                    "clipboard html was not found",
                )
            })?;

        Ok(value.to_string().into_bytes())
    })
}

/// Write one HTML payload through the macOS clipboard backend.
pub(crate) fn write_html_bytes(bytes: &[u8]) -> RuntimeResult<()> {
    let html = std::str::from_utf8(bytes).expect("validated html bytes should be utf8");

    with_general_pasteboard(|pasteboard| {
        let html = NSString::from_str(html);

        // clear stale formats before publishing the new html payload
        pasteboard.clearContents();

        if pasteboard.setString_forType(&html, unsafe { NSPasteboardTypeHTML }) {
            return Ok(());
        }

        Err(io_operation_error(
            CLIPBOARD_WRITE_BYTES_OPERATION,
            None,
            "pasteboard rejected html payload",
        ))
    })
}

/// Read one monotonic clipboard sequence from the macOS clipboard backend.
pub(crate) fn sequence() -> RuntimeResult<u64> {
    with_general_pasteboard(|pasteboard| {
        let change_count = pasteboard.changeCount();
        if change_count < 0 {
            return Err(io_operation_error(
                CLIPBOARD_SEQUENCE_OPERATION,
                None,
                "pasteboard returned one negative change count",
            ));
        }

        Ok(change_count as u64)
    })
}

/// Clear clipboard contents through the macOS clipboard backend.
pub(crate) fn clear() -> RuntimeResult<()> {
    with_general_pasteboard(|pasteboard| {
        pasteboard.clearContents();

        Ok(())
    })
}
