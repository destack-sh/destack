use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};

/// Build one document picker operation.
pub(crate) fn pick(
    options: DocumentPickOptionsValue,
) -> HostOperation<Vec<DocumentDescriptorValue>> {
    HostOperation::new(
        HostRequest::OsDocumentPick { options },
        decode::document_descriptors,
    )
}
