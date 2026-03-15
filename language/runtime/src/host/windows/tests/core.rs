use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::common::{
    HOST_DOCUMENT_PICK_OPERATION, normalized_document_extensions, validate_document_pick_options,
};
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Shared document-pick hook used by Windows tests.
pub(crate) type WindowsDocumentPickHook =
    fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>;

/// Return the shared Windows document-pick hook slot for tests.
fn windows_document_pick_hook_slot() -> &'static Mutex<Option<WindowsDocumentPickHook>> {
    static HOOK: OnceLock<Mutex<Option<WindowsDocumentPickHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install one Windows document-pick hook for tests.
pub(crate) fn set_test_pick_hook(hook: Option<WindowsDocumentPickHook>) {
    let mut slot = windows_document_pick_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Resolve the active Windows document-pick hook for tests.
fn require_test_pick_hook() -> RuntimeResult<WindowsDocumentPickHook> {
    let hook = windows_document_pick_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .to_owned();

    let Some(hook) = hook else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "document pick tests must install a picker hook before calling documentPick",
        ))
        .boxed());
    };

    Ok(hook)
}

/// Normalize Windows picker options for tests.
fn normalized_options(options: &DocumentPickOptionsValue) -> RuntimeResult<()> {
    validate_document_pick_options(options)?;

    let extensions = normalized_document_extensions(options)?;

    if options.allow_directories && !extensions.is_empty() {
        return Err(not_supported(HOST_DOCUMENT_PICK_OPERATION));
    }

    Ok(())
}

/// Pick documents from the Windows host request lane in tests.
pub(crate) fn pick_documents(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    normalized_options(options)?;
    let hook = require_test_pick_hook()?;

    hook(options.clone())
}
