use crate::diagnostic::RuntimeResult;
use crate::host::RequestContext;
#[cfg(windows)]
use crate::host::os::windows::tests::pick_documents as pick_documents_from_tests;
#[cfg(windows)]
pub(crate) use crate::host::os::windows::tests::set_windows_document_test_pick_hook;
#[cfg(not(windows))]
use crate::platform::core::not_supported;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Pick documents through the active Windows request lane in tests.
#[cfg(windows)]
pub(crate) fn pick_documents(
    context: &RequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_from_tests(context, options)
}

/// Fail closed for non-Windows test builds that compile the Windows host tree.
#[cfg(not(windows))]
pub(crate) fn pick_documents(
    _context: &RequestContext,
    _options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    Err(not_supported("destack.os.document.pick"))
}

/// Install one Windows document-pick hook for tests.
#[cfg(not(windows))]
pub(crate) fn set_windows_document_test_pick_hook(
    _hook: Option<fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>>,
) {
}
