use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::host::operation::contact as host_contact;
use crate::platform::VmAbiCodec;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::platform::os::{ContactPageVm, ContactVm};
use crate::runtime::BindingCallContext;

/// List host contacts through the active host session.
pub(crate) fn list(
    binding: &BindingCallContext,
    query: ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    binding.host().submit_operation(host_contact::list(query))
}

/// Search host contacts through the active host session.
pub(crate) fn search(
    binding: &BindingCallContext,
    query_text: &str,
    query: ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    binding
        .host()
        .submit_operation(host_contact::search(query_text.to_string(), query))
}

/// Read one host contact through the active host session.
pub(crate) fn read(binding: &BindingCallContext, id: &str) -> RuntimeResult<ContactValue> {
    binding
        .host()
        .submit_operation(host_contact::read(id.to_string()))
}

/// Create one host contact through the active host session.
pub(crate) fn create(
    binding: &BindingCallContext,
    contact: ContactDraftValue,
) -> RuntimeResult<String> {
    binding
        .host()
        .submit_operation(host_contact::create(contact))
}

/// Update one host contact through the active host session.
pub(crate) fn update(
    binding: &BindingCallContext,
    id: &str,
    contact: ContactDraftValue,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_contact::update(id.to_string(), contact))
}

/// Delete one host contact through the active host session.
pub(crate) fn delete(binding: &BindingCallContext, id: &str) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_contact::delete(id.to_string()))
}

/// Encode one contact page into one VM value.
pub(crate) fn page_vm(
    context: &mut vm::BindingContext<'_>,
    page: ContactPageValue,
) -> RuntimeResult<ContactPageVm> {
    ContactPageVm::from_value(&mut context.write(), page)
}

/// Encode one contact into one VM value.
pub(crate) fn contact_vm(
    context: &mut vm::BindingContext<'_>,
    contact: ContactValue,
) -> RuntimeResult<ContactVm> {
    ContactVm::from_value(&mut context.write(), contact)
}

/// Encode one identifier into one VM string handle.
pub(crate) fn id_vm(
    context: &mut vm::BindingContext<'_>,
    id: &str,
) -> RuntimeResult<vm::StringHandle> {
    <vm::StringHandle as VmAbiCodec>::from_value(&mut context.write(), id.to_string())
}
