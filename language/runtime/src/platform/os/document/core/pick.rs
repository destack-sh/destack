use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::document::state;
use crate::platform::os::{DocumentDescriptor, DocumentDescriptorVm};
use crate::platform::{NativeAbiCodec, VmAbiCodec, VmArray};
use crate::runtime::BindingCallContext;

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
    context: &mut vm::BindingContext<'_>,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let values = pick_values(binding, options)?;
    let mut descriptors = Vec::with_capacity(values.len());

    // encode one descriptor per picker result
    for value in values {
        let descriptor = DocumentDescriptorVm::from_value(&mut context.write(), value)?;
        descriptors.push(descriptor);
    }

    VmArray::from_values(&mut context.write(), &descriptors)
}

/// Submit one document pick operation and decode the raw values.
fn pick_values(
    binding: &BindingCallContext,
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    state::pick_values(binding, options)
}
