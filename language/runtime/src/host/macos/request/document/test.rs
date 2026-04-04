use crate::diagnostic::RuntimeResult;
use crate::host::RequestContext;
#[cfg(target_os = "macos")]
use crate::host::os::macos::tests::pick_documents as pick_documents_from_tests;
pub(crate) use crate::host::os::macos::tests::set_macos_document_test_pick_hook;
#[cfg(not(target_os = "macos"))]
use crate::platform::core::not_supported;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Pick documents through the active macOS request lane in tests.
#[cfg(target_os = "macos")]
pub(crate) fn pick_documents(
    context: &RequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_from_tests(context, options)
}

/// Fail closed for non-macOS test builds that compile the macOS host tree.
#[cfg(not(target_os = "macos"))]
pub(crate) fn pick_documents(
    _context: &RequestContext,
    _options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    Err(not_supported("destack.os.document.pick"))
}

/// Install one macOS document-pick hook for tests.
#[cfg(not(target_os = "macos"))]
pub(crate) fn set_macos_document_test_pick_hook(
    _hook: Option<fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>>,
) {
}
