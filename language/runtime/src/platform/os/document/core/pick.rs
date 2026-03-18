use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::VmArray;
use crate::platform::core::{NativeAbiCodec, VmAbiCodec};
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::{DocumentDescriptor, DocumentDescriptorVm};
use crate::runtime::BindingCallContext;

use crate::host::operation::document as host_document;

/// Import selected documents into app-owned storage.
pub(crate) fn import_native(
    binding: &BindingCallContext,
    documents: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<Vec<DocumentDescriptor>> {
    let values = import_values(binding, documents)?;
    let descriptors = values
        .into_iter()
        .map(|value| DocumentDescriptor::from_value(binding, value))
        .collect();

    Ok(descriptors)
}

/// Import selected documents into app-owned storage for the VM ABI.
pub(crate) fn import_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    documents: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let values = import_values(binding, documents)?;
    let mut descriptors = Vec::with_capacity(values.len());

    // encode one descriptor per imported document
    for value in values {
        let descriptor = DocumentDescriptorVm::from_value(context, value)?;
        descriptors.push(descriptor);
    }

    VmArray::from_values(context, &descriptors)
}

/// Pick documents from host UI.
pub(crate) fn pick_native(
    binding: &BindingCallContext,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptor>> {
    let values = pick_values(binding, options)?;
    let descriptors = values
        .into_iter()
        .map(|value| DocumentDescriptor::from_value(binding, value))
        .collect();

    Ok(descriptors)
}

/// Pick documents from host UI for the VM ABI.
pub(crate) fn pick_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let values = pick_values(binding, options)?;
    let mut descriptors = Vec::with_capacity(values.len());

    // encode one descriptor per picker result
    for value in values {
        let descriptor = DocumentDescriptorVm::from_value(context, value)?;
        descriptors.push(descriptor);
    }

    VmArray::from_values(context, &descriptors)
}

/// Submit one document import operation and decode the raw values.
fn import_values(
    binding: &BindingCallContext,
    documents: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    binding
        .host()
        .submit_operation(host_document::import(documents))
}

/// Submit one document pick operation and decode the raw values.
fn pick_values(
    binding: &BindingCallContext,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    binding
        .host()
        .submit_operation(host_document::pick(options))
}
