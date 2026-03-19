use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};
use objc2_foundation::{NSArray, NSCopying, NSData, NSString};

use crate::diagnostic::RuntimeResult;
use crate::host::apple::execution::call_process_main_context_if_needed;
use crate::platform::core::{invalid_argument, io_not_found, io_operation_error};
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
        let text = ProtocolObject::from_retained(text.copy());
        let objects = NSArray::from_retained_slice(&[text]);

        // publish plain text through the AppKit pasteboard-writing path
        if pasteboard.writeObjects(&objects) {
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
        // prefer raw typed data for html payloads
        if let Some(value) = pasteboard.dataForType(unsafe { NSPasteboardTypeHTML }) {
            return Ok(value.to_vec());
        }

        // fall back to string decoding for older writers
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
    let _html = std::str::from_utf8(bytes)
        .map_err(|_| invalid_argument("bytes", "clipboard html payload must be utf8"))?;

    with_general_pasteboard(|pasteboard| {
        let html = NSData::with_bytes(bytes);

        // clear stale formats before publishing the new html payload
        pasteboard.clearContents();

        // publish html through one explicit typed data payload
        if pasteboard.setData_forType(Some(&html), unsafe { NSPasteboardTypeHTML }) {
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
        // clear the shared pasteboard contents in place
        pasteboard.clearContents();

        Ok(())
    })
}
