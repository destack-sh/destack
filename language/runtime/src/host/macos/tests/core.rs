use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::common::{normalized_document_extensions, validate_document_pick_options};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Shared document-pick hook used by macOS tests.
pub(crate) type MacosDocumentPickHook =
    fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>;

/// Return the shared macOS document-pick hook slot for tests.
fn macos_document_pick_hook_slot() -> &'static Mutex<Option<MacosDocumentPickHook>> {
    static HOOK: OnceLock<Mutex<Option<MacosDocumentPickHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install one macOS document-pick hook for tests.
pub(crate) fn set_test_pick_hook(hook: Option<MacosDocumentPickHook>) {
    let mut slot = macos_document_pick_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Resolve the active macOS document-pick hook for tests.
fn require_test_pick_hook() -> RuntimeResult<MacosDocumentPickHook> {
    let hook = macos_document_pick_hook_slot()
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

/// Pick documents from the macOS host request lane in tests.
pub(crate) fn pick_documents(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    validate_document_pick_options(options)?;

    let _ = normalized_document_extensions(options)?;
    let hook = require_test_pick_hook()?;

    hook(options.clone())
}
