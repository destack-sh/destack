use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::contact::{
    decode_contact, decode_contact_page, encode_contact_draft, encode_contact_query,
};
use crate::host::android::abi::contact::{
    destack_host_android_contact_create, destack_host_android_contact_delete,
    destack_host_android_contact_list, destack_host_android_contact_read,
    destack_host_android_contact_search, destack_host_android_contact_update,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::os::abi_generated::{
    Contact, ContactDraftValue, ContactPage, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::runtime::{BindingCallContext, NativeStringRef};

/// Return one Android contact request outcome when supported.
pub(crate) fn submit_contact_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsContactList { query } => {
            let page = submit_contact_list(runtime_id, request.operation_name(), query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }
        HostRequest::OsContactSearch { query_text, query } => {
            let page =
                submit_contact_search(runtime_id, request.operation_name(), query_text, query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }
        HostRequest::OsContactRead { id } => {
            let contact = submit_contact_read(runtime_id, request.operation_name(), id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Contact(contact),
            )))
        }
        HostRequest::OsContactCreate { contact } => {
            let id = submit_contact_create(runtime_id, request.operation_name(), contact)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsContactUpdate { id, contact } => {
            submit_contact_update(runtime_id, request.operation_name(), id, contact)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsContactDelete { id } => {
            let call_status = unsafe {
                destack_host_android_contact_delete(runtime_id, NativeStringRef::from(id))
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Submit one Android contact-list request.
fn submit_contact_list(
    runtime_id: u64,
    operation: &'static str,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let query = encode_contact_query(&binding, query);
    let mut output_page = MaybeUninit::<ContactPage>::uninit();
    let status =
        unsafe { destack_host_android_contact_list(runtime_id, query, output_page.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let output_page = unsafe { output_page.assume_init() };
    unsafe { decode_contact_page(output_page) }
}

/// Submit one Android contact-search request.
fn submit_contact_search(
    runtime_id: u64,
    operation: &'static str,
    query_text: &str,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let query = encode_contact_query(&binding, query);
    let mut output_page = MaybeUninit::<ContactPage>::uninit();
    let status = unsafe {
        destack_host_android_contact_search(
            runtime_id,
            NativeStringRef::from(query_text),
            query,
            output_page.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let output_page = unsafe { output_page.assume_init() };
    unsafe { decode_contact_page(output_page) }
}

/// Submit one Android contact-read request.
fn submit_contact_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<ContactValue> {
    let mut output_contact = MaybeUninit::<Contact>::uninit();
    let status = unsafe {
        destack_host_android_contact_read(
            runtime_id,
            NativeStringRef::from(id),
            output_contact.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let output_contact = unsafe { output_contact.assume_init() };
    unsafe { decode_contact(output_contact) }
}

/// Submit one Android contact-create request.
fn submit_contact_create(
    runtime_id: u64,
    operation: &'static str,
    contact: &ContactDraftValue,
) -> RuntimeResult<String> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let draft = encode_contact_draft(&binding, contact);
    let mut output_id = MaybeUninit::<NativeStringRef>::uninit();
    let status =
        unsafe { destack_host_android_contact_create(runtime_id, draft, output_id.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let output_id = unsafe { output_id.assume_init() };
    Ok(unsafe { output_id.as_str()? }.to_string())
}

/// Submit one Android contact-update request.
fn submit_contact_update(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
    contact: &ContactDraftValue,
) -> RuntimeResult<()> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let draft = encode_contact_draft(&binding, contact);
    let call_status = unsafe {
        destack_host_android_contact_update(runtime_id, NativeStringRef::from(id), draft)
    };
    decode_callback_host_status(call_status, operation)
}
