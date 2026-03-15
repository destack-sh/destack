#[cfg(all(not(test), windows))]
mod submit;

#[cfg(test)]
use crate::diagnostic::RuntimeResult;
#[cfg(test)]
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

#[cfg(all(test, windows))]
use crate::host::windows::tests::pick_documents as pick_documents_from_tests;
#[cfg(all(test, windows))]
pub(crate) use crate::host::windows::tests::set_test_pick_hook;

/// Pick documents through the active Windows request lane.
#[cfg(all(not(test), windows))]
pub(crate) use submit::pick_documents;

/// Pick documents through the active Windows request lane in tests.
#[cfg(all(test, windows))]
pub(crate) fn pick_documents(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    pick_documents_from_tests(options)
}
