#[cfg(test)]
use crate::diagnostic::RuntimeResult;
#[cfg(test)]
use crate::host::set_windows_document_test_pick_hook;
#[cfg(test)]
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Install one Windows document-pick hook for tests.
#[cfg(test)]
pub(crate) fn set_test_pick_hook(
    hook: Option<fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>>,
) {
    set_windows_document_test_pick_hook(hook);
}
