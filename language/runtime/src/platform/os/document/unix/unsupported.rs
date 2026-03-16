use crate::diagnostic::RuntimeResult;
use crate::platform::core::not_supported;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::document::core::DOCUMENT_PICK_OPERATION;

/// Pick documents from parked Unix document backends.
#[allow(dead_code)]
pub(crate) fn pick(
    _options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    Err(not_supported(DOCUMENT_PICK_OPERATION))
}
