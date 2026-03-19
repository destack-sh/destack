use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{NSArray, NSString};
use objc2_uniform_type_identifiers::UTType;

use crate::diagnostic::RuntimeResult;
use crate::host::app::document::pick::{
    HOST_DOCUMENT_PICK_OPERATION, document_descriptor_value_from_path,
    validate_document_pick_options, validated_document_content_types,
    validated_document_extensions,
};
use crate::host::apple::execution::with_process_main_context_marker_if_needed;
use crate::host::core::HostRequestContext;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Build one Cocoa array for allowed content types.
fn allowed_content_types(
    content_types: &[String],
    extensions: &[String],
) -> RuntimeResult<Option<objc2::rc::Retained<NSArray<UTType>>>> {
    if content_types
        .iter()
        .any(|content_type| content_type == "*/*")
    {
        return Ok(None);
    }

    let mut values = content_types
        .iter()
        .map(|content_type| {
            let content_type = NSString::from_str(content_type);

            UTType::typeWithMIMEType(&content_type).ok_or_else(|| {
                io_operation_error(
                    HOST_DOCUMENT_PICK_OPERATION,
                    Some(PlatformErrorCode::IoInvalidData),
                    "document picker content type is not recognized on this macOS backend",
                )
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    let extension_types = extensions
        .iter()
        .map(|extension| {
            let extension = NSString::from_str(extension);

            UTType::typeWithFilenameExtension(&extension).ok_or_else(|| {
                io_operation_error(
                    HOST_DOCUMENT_PICK_OPERATION,
                    Some(PlatformErrorCode::IoInvalidData),
                    "document picker extension is not recognized on this macOS backend",
                )
            })
        })
        .collect::<RuntimeResult<Vec<_>>>()?;

    values.extend(extension_types);

    if values.is_empty() {
        return Ok(None);
    }

    let references = values
        .iter()
        .map(|value| value.as_ref())
        .collect::<Vec<_>>();

    Ok(Some(NSArray::from_slice(&references)))
}

/// Open one macOS document picker on the process main thread.
fn pick_documents_on_main(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    validate_document_pick_options(options)?;

    let content_types = validated_document_content_types(options)?;
    let extensions = validated_document_extensions(options)?;

    with_process_main_context_marker_if_needed(|mtm| {
        let panel = NSOpenPanel::openPanel(mtm);

        // configure one picker that can optionally return multiple files or directories
        panel.setCanChooseDirectories(options.allow_directories);
        panel.setCanChooseFiles(true);
        panel.setAllowsMultipleSelection(options.multiple);

        if let Some(allowed_content_types) = allowed_content_types(&content_types, &extensions)? {
            panel.setAllowedContentTypes(&allowed_content_types);
        }

        let response = panel.runModal();
        if response != NSModalResponseOK {
            return Ok(Vec::new());
        }

        let urls = panel.URLs();
        let mut descriptors = Vec::with_capacity(urls.len());

        // decode one descriptor per selected file url
        for url in urls.iter() {
            let path = url.path().ok_or_else(|| {
                io_operation_error(
                    HOST_DOCUMENT_PICK_OPERATION,
                    Some(PlatformErrorCode::IoInvalidData),
                    "document picker returned one url without one local path",
                )
            })?;
            let path = std::path::PathBuf::from(path.to_string());
            let descriptor = document_descriptor_value_from_path(&path)?;

            descriptors.push(descriptor);
        }

        Ok(descriptors)
    })
}

/// Pick documents from the macOS host request lane.
pub(crate) fn pick_documents(
    _context: &HostRequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_on_main(options)
}
