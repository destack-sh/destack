use super::tests::{
    bytes_harness_value, decode_clipboard_bytes, decode_clipboard_text, restore_clipboard,
    snapshot_clipboard, string_harness_value,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::ClipboardBinaryFormat;
use crate::platform::os::tests::with_harness_context;
#[cfg(target_os = "macos")]
use crate::tests::execution::run_execution_case_or_return;
use crate::tests::platform::assert_runtime_error_code;

/// Return whether one clipboard roundtrip test should defer to the execution harness.
fn should_skip_clipboard_roundtrip(test_name: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        run_execution_case_or_return(test_name)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = test_name;

        false
    }
}

/// Verify clipboard text and `TextUtf8` bytes roundtrip on supported hosts.
#[cfg_attr(test, test)]
pub(crate) fn test_clipboard_text_roundtrip() {
    if should_skip_clipboard_roundtrip(concat!(
        "destack_runtime::platform::os::clipboard::tests::",
        stringify!(test_clipboard_text_roundtrip)
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let snapshot = snapshot_clipboard(&mut context)?;
        let initial_sequence = context.destack_os_clipboard_sequence()?;
        let write_text = string_harness_value(&mut context, "destack clipboard roundtrip");
        context.destack_os_clipboard_write_text(write_text)?;

        let has_text = context.destack_os_clipboard_has_text()?;
        let read_text = context.destack_os_clipboard_read_text()?;
        let read_text = decode_clipboard_text(&mut context, read_text)?;
        let read_bytes =
            context.destack_os_clipboard_read_bytes(ClipboardBinaryFormat::TextUtf8)?;
        let read_bytes = decode_clipboard_bytes(&mut context, read_bytes)?;
        let updated_sequence = context.destack_os_clipboard_sequence()?;

        assert!(has_text);
        assert_eq!(read_text, "destack clipboard roundtrip");
        assert_eq!(read_bytes, b"destack clipboard roundtrip");
        assert!(updated_sequence >= initial_sequence);

        restore_clipboard(&mut context, snapshot)?;
        Ok(())
    });
}

/// Verify clipboard html bytes roundtrip on supported hosts.
#[cfg_attr(test, test)]
pub(crate) fn test_clipboard_html_roundtrip() {
    if should_skip_clipboard_roundtrip(concat!(
        "destack_runtime::platform::os::clipboard::tests::",
        stringify!(test_clipboard_html_roundtrip)
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let snapshot = snapshot_clipboard(&mut context)?;
        let html = b"<!doctype html><b>destack</b>";
        let write_html = bytes_harness_value(&mut context, html)?;
        context.destack_os_clipboard_write_bytes(ClipboardBinaryFormat::Html, write_html)?;

        let read_html = context.destack_os_clipboard_read_bytes(ClipboardBinaryFormat::Html)?;
        let read_html = decode_clipboard_bytes(&mut context, read_html)?;
        assert_eq!(read_html, html);

        restore_clipboard(&mut context, snapshot)?;
        Ok(())
    });
}

/// Verify clipboard clear empties text availability on supported hosts.
#[cfg_attr(test, test)]
pub(crate) fn test_clipboard_clear_resets_text_payload() {
    if should_skip_clipboard_roundtrip(concat!(
        "destack_runtime::platform::os::clipboard::tests::",
        stringify!(test_clipboard_clear_resets_text_payload)
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let snapshot = snapshot_clipboard(&mut context)?;
        let write_text = string_harness_value(&mut context, "destack clipboard clear");
        context.destack_os_clipboard_write_text(write_text)?;
        context.destack_os_clipboard_clear()?;

        let has_text = context.destack_os_clipboard_has_text()?;
        assert!(!has_text);

        let error = match context.destack_os_clipboard_read_text() {
            Ok(_) => panic!("clipboardReadText should report ioNotFound after clear"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::IoNotFound);

        restore_clipboard(&mut context, snapshot)?;
        Ok(())
    });
}
