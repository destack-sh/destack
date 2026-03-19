use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::os::DocumentAccess;
use crate::platform::os::abi_generated::{
    DocumentAccessGrantValue, DocumentDescriptorValue, DocumentPickOptionsValue,
};

/// Build one document picker operation.
pub(crate) fn pick(
    options: DocumentPickOptionsValue,
) -> HostOperation<Vec<DocumentDescriptorValue>> {
    HostOperation::new(
        HostRequest::OsDocumentPick { options },
        decode::document_descriptors,
    )
}

/// Build one document import operation.
pub(crate) fn import(
    documents: Vec<DocumentDescriptorValue>,
) -> HostOperation<Vec<DocumentDescriptorValue>> {
    HostOperation::new(
        HostRequest::OsDocumentImport { documents },
        decode::document_descriptors,
    )
}

/// Build one document-access persist operation.
pub(crate) fn access_persist(
    documents: Vec<DocumentDescriptorValue>,
    access: DocumentAccess,
) -> HostOperation<Vec<DocumentAccessGrantValue>> {
    HostOperation::new(
        HostRequest::OsDocumentAccessPersist { documents, access },
        decode::document_access_grants,
    )
}

/// Build one document-access list operation.
pub(crate) fn access_list() -> HostOperation<Vec<DocumentAccessGrantValue>> {
    HostOperation::new(
        HostRequest::OsDocumentAccessList,
        decode::document_access_grants,
    )
}

/// Build one document-access revoke operation.
pub(crate) fn access_revoke(ids: Vec<String>) -> HostOperation<u32> {
    HostOperation::new(
        HostRequest::OsDocumentAccessRevoke { ids },
        decode::u32_value,
    )
}
