use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::VmArray;
use crate::platform::core::{NativeAbiCodec, VmAbiCodec};
use crate::platform::os::abi_generated::{DocumentAccessGrantValue, DocumentDescriptorValue};
use crate::platform::os::{DocumentAccess, DocumentAccessGrant, DocumentAccessGrantVm};
use crate::platform::resource::DocumentHandle;
use crate::runtime::BindingCallContext;

use crate::host::operation::document as host_document;

use super::{
    DOCUMENT_ACCESS_OPEN_OPERATION, local_path_from_document_descriptor, open_document_path,
    validate_grant_access,
};

/// Open one persisted document-access grant.
pub(crate) fn access_open(
    binding: &BindingCallContext,
    id: &str,
    access: DocumentAccess,
) -> RuntimeResult<DocumentHandle> {
    let grant = access_grant_value(binding, id)?;
    validate_grant_access(&grant, access)?;
    let path = local_path_from_document_descriptor(&grant.document, "id")?;

    open_document_path(binding, path, access, DOCUMENT_ACCESS_OPEN_OPERATION)
}

/// List persisted document-access grants.
pub(crate) fn access_list_native(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<DocumentAccessGrant>> {
    let values = access_list_values(binding)?;
    let grants = values
        .into_iter()
        .map(|value| DocumentAccessGrant::from_value(binding, value))
        .collect();

    Ok(grants)
}

/// List persisted document-access grants for the VM ABI.
pub(crate) fn access_list_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<DocumentAccessGrantVm>> {
    let values = access_list_values(binding)?;
    let mut grants = Vec::with_capacity(values.len());

    // encode one grant per persisted document-access entry
    for value in values {
        let grant = DocumentAccessGrantVm::from_value(context, value)?;
        grants.push(grant);
    }

    VmArray::from_values(context, &grants)
}

/// Persist document-access grants for one document set.
pub(crate) fn access_persist_native(
    binding: &BindingCallContext,
    documents: Vec<DocumentDescriptorValue>,
    access: DocumentAccess,
) -> RuntimeResult<Vec<DocumentAccessGrant>> {
    let values = access_persist_values(binding, documents, access)?;
    let grants = values
        .into_iter()
        .map(|value| DocumentAccessGrant::from_value(binding, value))
        .collect();

    Ok(grants)
}

/// Persist document-access grants for one document set for the VM ABI.
pub(crate) fn access_persist_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    documents: Vec<DocumentDescriptorValue>,
    access: DocumentAccess,
) -> RuntimeResult<VmArray<DocumentAccessGrantVm>> {
    let values = access_persist_values(binding, documents, access)?;
    let mut grants = Vec::with_capacity(values.len());

    // encode one grant per persisted document-access result
    for value in values {
        let grant = DocumentAccessGrantVm::from_value(context, value)?;
        grants.push(grant);
    }

    VmArray::from_values(context, &grants)
}

/// Revoke persisted document-access grants.
pub(crate) fn access_revoke(binding: &BindingCallContext, ids: Vec<String>) -> RuntimeResult<u32> {
    binding
        .host()
        .submit_operation(host_document::access_revoke(ids))
}

/// Submit one document access-list operation and decode the raw values.
fn access_list_values(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<DocumentAccessGrantValue>> {
    binding
        .host()
        .submit_operation(host_document::access_list())
}

/// Submit one document access-persist operation and decode the raw values.
fn access_persist_values(
    binding: &BindingCallContext,
    documents: Vec<DocumentDescriptorValue>,
    access: DocumentAccess,
) -> RuntimeResult<Vec<DocumentAccessGrantValue>> {
    binding
        .host()
        .submit_operation(host_document::access_persist(documents, access))
}

/// Resolve one persisted document-access grant by identifier.
fn access_grant_value(
    binding: &BindingCallContext,
    id: &str,
) -> RuntimeResult<DocumentAccessGrantValue> {
    let grants = access_list_values(binding)?;

    grants
        .into_iter()
        .find(|grant| grant.id == id)
        .ok_or_else(|| {
            crate::platform::core::io_not_found(
                DOCUMENT_ACCESS_OPEN_OPERATION,
                "unknown document access grant",
            )
        })
}
