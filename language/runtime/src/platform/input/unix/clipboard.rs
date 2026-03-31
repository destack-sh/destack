use crate::diagnostic::RuntimeResult;
#[cfg(not(target_os = "macos"))]
use crate::platform::core::not_supported;
#[cfg(not(target_os = "macos"))]
use crate::platform::input::core::{CLIPBOARD_CLEAR_OPERATION, CLIPBOARD_HAS_TEXT_OPERATION};
use crate::platform::input::core::{
    CLIPBOARD_READ_ITEM_BYTES_OPERATION, CLIPBOARD_READ_TEXT_OPERATION,
    CLIPBOARD_SEQUENCE_OPERATION, CLIPBOARD_WRITE_ITEMS_OPERATION, CLIPBOARD_WRITE_TEXT_OPERATION,
};

#[cfg(target_os = "macos")]
use objc2::runtime::ProtocolObject;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSCopying, NSData, NSString};

#[cfg(target_os = "macos")]
use crate::host::apple::core::execution::call_process_main_context_if_needed;
#[cfg(target_os = "macos")]
use crate::platform::core::{invalid_argument, io_not_found, io_operation_error};

#[cfg(target_os = "macos")]
fn with_general_pasteboard<R: Send>(
    callback: impl FnOnce(&NSPasteboard) -> RuntimeResult<R> + Send,
) -> RuntimeResult<R> {
    call_process_main_context_if_needed(|| {
        let pasteboard = NSPasteboard::generalPasteboard();

        callback(&pasteboard)
    })
}

/// Query whether one text payload exists on the Unix clipboard backend.
pub(crate) fn has_text() -> RuntimeResult<bool> {
    #[cfg(target_os = "macos")]
    {
        return with_general_pasteboard(|pasteboard| {
            Ok(pasteboard
                .stringForType(unsafe { NSPasteboardTypeString })
                .is_some())
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(not_supported(CLIPBOARD_HAS_TEXT_OPERATION))
    }
}

/// Read one text payload from the Unix clipboard backend.
pub(crate) fn read_text() -> RuntimeResult<String> {
    #[cfg(target_os = "macos")]
    {
        return with_general_pasteboard(|pasteboard| {
            let value = pasteboard
                .stringForType(unsafe { NSPasteboardTypeString })
                .ok_or_else(|| {
                    io_not_found(
                        CLIPBOARD_READ_TEXT_OPERATION,
                        "clipboard text was not found",
                    )
                })?;

            Ok(value.to_string())
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(not_supported(CLIPBOARD_READ_TEXT_OPERATION))
    }
}

/// Write one text payload through the Unix clipboard backend.
pub(crate) fn write_text(text: &str) -> RuntimeResult<()> {
    #[cfg(target_os = "macos")]
    {
        return with_general_pasteboard(|pasteboard| {
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
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;

        Err(not_supported(CLIPBOARD_WRITE_TEXT_OPERATION))
    }
}

/// Read one HTML payload from the Unix clipboard backend.
pub(crate) fn read_html_bytes() -> RuntimeResult<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        return with_general_pasteboard(|pasteboard| {
            // prefer raw typed data for html payloads
            if let Some(value) = pasteboard.dataForType(unsafe { NSPasteboardTypeHTML }) {
                return Ok(value.to_vec());
            }

            // fall back to string decoding for older writers
            let value = pasteboard
                .stringForType(unsafe { NSPasteboardTypeHTML })
                .ok_or_else(|| {
                    io_not_found(
                        CLIPBOARD_READ_ITEM_BYTES_OPERATION,
                        "clipboard html was not found",
                    )
                })?;

            Ok(value.to_string().into_bytes())
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(not_supported(CLIPBOARD_READ_ITEM_BYTES_OPERATION))
    }
}

/// Write one HTML payload through the Unix clipboard backend.
pub(crate) fn write_html_bytes(bytes: &[u8]) -> RuntimeResult<()> {
    #[cfg(target_os = "macos")]
    {
        let _html = std::str::from_utf8(bytes)
            .map_err(|_| invalid_argument("bytes", "clipboard html payload must be utf8"))?;

        return with_general_pasteboard(|pasteboard| {
            let html = NSData::with_bytes(bytes);

            // clear stale formats before publishing the new html payload
            pasteboard.clearContents();

            // publish html through one explicit typed data payload
            if pasteboard.setData_forType(Some(&html), unsafe { NSPasteboardTypeHTML }) {
                return Ok(());
            }

            Err(io_operation_error(
                CLIPBOARD_WRITE_ITEMS_OPERATION,
                None,
                "pasteboard rejected html payload",
            ))
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = bytes;

        Err(not_supported(CLIPBOARD_WRITE_ITEMS_OPERATION))
    }
}

/// Read one monotonic clipboard sequence from the Unix clipboard backend.
pub(crate) fn sequence() -> RuntimeResult<u64> {
    #[cfg(target_os = "macos")]
    {
        return with_general_pasteboard(|pasteboard| {
            let change_count = pasteboard.changeCount();
            if change_count < 0 {
                return Err(io_operation_error(
                    CLIPBOARD_SEQUENCE_OPERATION,
                    None,
                    "pasteboard returned one negative change count",
                ));
            }

            Ok(change_count as u64)
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(not_supported(CLIPBOARD_SEQUENCE_OPERATION))
    }
}

/// Clear clipboard contents through the Unix clipboard backend.
pub(crate) fn clear() -> RuntimeResult<()> {
    #[cfg(target_os = "macos")]
    {
        return with_general_pasteboard(|pasteboard| {
            // clear the shared pasteboard contents in place
            pasteboard.clearContents();

            Ok(())
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(not_supported(CLIPBOARD_CLEAR_OPERATION))
    }
}
