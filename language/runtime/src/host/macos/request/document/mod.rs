#[cfg(all(not(test), target_os = "macos"))]
mod submit;

#[cfg(test)]
use crate::diagnostic::RuntimeResult;
#[cfg(test)]
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

#[cfg(all(test, target_os = "macos"))]
use crate::host::macos::tests::pick_documents as pick_documents_from_tests;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use crate::host::macos::tests::set_test_pick_hook;
#[cfg(all(test, not(target_os = "macos")))]
use crate::platform::core::not_supported;

/// Pick documents through the active macOS request lane.
#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use submit::pick_documents;

/// Pick documents through the active macOS request lane in tests.
#[cfg(all(test, target_os = "macos"))]
pub(crate) fn pick_documents(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_from_tests(options)
}

/// Fail closed for non-macOS test builds that compile the macOS host tree.
#[cfg(all(test, not(target_os = "macos")))]
pub(crate) fn pick_documents(
    _options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    Err(not_supported("destack.os.document.pick"))
}
