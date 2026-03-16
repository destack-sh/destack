use crate::diagnostic::RuntimeResult;
use crate::platform::core::{invalid_argument, not_supported};
use crate::platform::os::abi_generated::DocumentPickOptionsValue;

/// Canonical operation name for `documentPick`.
pub(crate) const HOST_DOCUMENT_PICK_OPERATION: &str = "destack.os.document.pick";

/// Validate document-picker options shared across host backends.
pub(crate) fn validate_document_pick_options(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<()> {
    if !options.mime_types.is_empty() {
        return Err(not_supported(HOST_DOCUMENT_PICK_OPERATION));
    }

    if options.copy_to_sandbox {
        return Err(not_supported(HOST_DOCUMENT_PICK_OPERATION));
    }

    Ok(())
}

/// Normalize shared document-picker extension filters.
pub(crate) fn normalized_document_extensions(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<String>> {
    let mut extensions = Vec::with_capacity(options.extensions.len());

    // validate one extension filter at a time
    for extension in &options.extensions {
        if extension.is_empty() {
            return Err(invalid_argument(
                "options.extensions",
                "document picker extensions must not be empty",
            ));
        }

        if extension.starts_with('.') {
            return Err(invalid_argument(
                "options.extensions",
                "document picker extensions must not include a leading dot",
            ));
        }

        if extension.contains('/') || extension.contains('\\') {
            return Err(invalid_argument(
                "options.extensions",
                "document picker extensions must not contain path separators",
            ));
        }

        extensions.push(extension.clone());
    }

    Ok(extensions)
}

/// Percent-encode one byte slice for a URI payload.
#[cfg(any(all(not(test), target_os = "macos"), all(not(test), windows)))]
pub(crate) fn percent_encode_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len());

    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            output.push(*byte as char);
            continue;
        }

        output.push('%');
        output.push_str(&format!("{byte:02X}"));
    }

    output
}
