use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRequestContext;
#[cfg(windows)]
use crate::host::windows::tests::pick_documents as pick_documents_from_tests;
pub(crate) use crate::host::windows::tests::set_windows_document_test_pick_hook;
#[cfg(not(windows))]
use crate::platform::core::not_supported;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Pick documents through the active Windows request lane in tests.
#[cfg(windows)]
pub(crate) fn pick_documents(
    context: &HostRequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_from_tests(context, options)
}

/// Fail closed for non-Windows test builds that compile the Windows host tree.
#[cfg(not(windows))]
pub(crate) fn pick_documents(
    _context: &HostRequestContext,
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
