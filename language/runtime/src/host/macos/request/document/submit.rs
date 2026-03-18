use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{NSArray, NSString};

use crate::diagnostic::RuntimeResult;
use crate::host::app::document::pick::{
    HOST_DOCUMENT_PICK_OPERATION, document_descriptor_value_from_path,
    normalized_document_extensions, validate_document_pick_options,
};
use crate::host::apple::execution::with_process_main_context_marker_if_needed;
use crate::host::core::HostRequestContext;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
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
    _context: &HostRequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_on_main(options)
}
