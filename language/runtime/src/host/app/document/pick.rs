use crate::diagnostic::RuntimeResult;
use crate::platform::core::invalid_argument;
use crate::platform::os::abi_generated::DocumentPickOptionsValue;

pub(crate) use super::storage::document_descriptor_value_from_path;

/// Canonical operation name for `documentPick`.
#[allow(dead_code)]
pub(crate) const HOST_DOCUMENT_PICK_OPERATION: &str = "destack.os.document.pick";
/// Validate document-picker options shared across host backends.
pub(crate) fn validate_document_pick_options(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<()> {
    // empty mime-type filters are meaningless
    for mime_type in &options.mime_types {
        if mime_type.trim().is_empty() {
            return Err(invalid_argument(
                "options.mimeTypes",
                "document picker mime types must not be empty",
            ));
        }
    }

    Ok(())
}

/// Normalize shared document-picker extension filters.
pub(crate) fn validated_document_extensions(
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

        push_document_extension(&mut extensions, extension.to_ascii_lowercase());
    }

    Ok(extensions)
}

/// Normalize shared document-picker content-type filters.
pub(crate) fn validated_document_content_types(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<String>> {
    let mut mime_types = Vec::with_capacity(options.mime_types.len());

    // validate one mime-type filter at a time
    for mime_type in &options.mime_types {
        if mime_types.iter().any(|existing| existing == mime_type) {
            continue;
        }

        mime_types.push(mime_type.clone());
    }

    Ok(mime_types)
}

/// Push one normalized extension when it is not already present.
fn push_document_extension(extensions: &mut Vec<String>, extension: String) {
    if extensions.iter().any(|existing| existing == &extension) {
        return;
    }

    extensions.push(extension);
}
