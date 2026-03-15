use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{NSArray, NSString};
use std::os::unix::ffi::OsStrExt;
use std::time::UNIX_EPOCH;

use crate::diagnostic::RuntimeResult;
use crate::host::apple::execution::with_process_main_context_marker_if_needed;
use crate::host::common::{
    HOST_DOCUMENT_PICK_OPERATION, normalized_document_extensions, percent_encode_bytes,
    validate_document_pick_options,
};
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathValue, PathBytesValue};
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Build one Cocoa array for allowed extension filters.
fn allowed_file_types(extensions: &[String]) -> Option<objc2::rc::Retained<NSArray<NSString>>> {
    if extensions.is_empty() {
        return None;
    }

    let values = extensions
        .iter()
        .map(|extension| NSString::from_str(extension))
        .collect::<Vec<_>>();
    let references = values
        .iter()
        .map(|value| value.as_ref())
        .collect::<Vec<_>>();

    Some(NSArray::from_slice(&references))
}

/// Build one document descriptor from one local path.
fn document_descriptor_value_from_path(
    path: &std::path::Path,
) -> RuntimeResult<DocumentDescriptorValue> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        io_operation_error(
            HOST_DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoNotFound),
            format!("document path metadata failed: {error}"),
        )
    })?;
    let is_directory = metadata.is_dir();
    let modified_unix_nanos = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);

    Ok(DocumentDescriptorValue {
        uri: file_uri_from_path(path),
        local_path: Some(os_path_value_from_path(path)),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
        size_bytes: (!is_directory).then_some(metadata.len()),
        mime_type: None,
        is_directory,
        modified_unix_ns: Some(modified_unix_nanos),
    })
}

/// Encode one local host path into one `file://` URI.
fn file_uri_from_path(path: &std::path::Path) -> String {
    let bytes = path.as_os_str().as_bytes();
    let payload = percent_encode_bytes(bytes);

    format!("file://{payload}")
}

/// Encode one local host path into one runtime path value.
fn os_path_value_from_path(path: &std::path::Path) -> OsPathValue {
    let bytes = path.as_os_str().as_bytes().to_vec();

    OsPathValue::OsPathBytes(OsPathBytesValue {
        kind: "bytes".to_string(),
        bytes: PathBytesValue(bytes),
    })
}

/// Open one macOS document picker on the process main thread.
fn pick_documents_on_main(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    validate_document_pick_options(options)?;

    let extensions = normalized_document_extensions(options)?;

    with_process_main_context_marker_if_needed(|mtm| {
        let panel = NSOpenPanel::openPanel(mtm);

        // configure one picker that can optionally return multiple files or directories
        panel.setCanChooseDirectories(options.allow_directories);
        panel.setCanChooseFiles(true);
        panel.setAllowsMultipleSelection(options.multiple);

        #[allow(deprecated)]
        if let Some(allowed_file_types) = allowed_file_types(&extensions) {
            panel.setAllowedFileTypes(Some(&allowed_file_types));
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
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_on_main(options)
}
