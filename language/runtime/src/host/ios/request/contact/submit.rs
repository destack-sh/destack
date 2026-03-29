use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::contact::{
    HostContactCreateResponse, HostContactPageResponse, HostContactResponse,
    decode_contact_create_response, decode_contact_page_response, decode_contact_response,
    encode_contact_draft, encode_contact_query,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::host::ios::abi::contact::{
    destack_host_ios_contact_create, destack_host_ios_contact_delete_contact,
    destack_host_ios_contact_list, destack_host_ios_contact_read, destack_host_ios_contact_search,
    destack_host_ios_contact_update,
};
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::runtime::BindingCallContext;

/// Return one iOS contact request outcome when supported.
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
                destack_host_ios_contact_delete_contact(runtime_id, NativeStringRef::from(id))
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Submit one iOS contact-list request.
fn submit_contact_list(
    runtime_id: u64,
    operation: &'static str,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let query = encode_contact_query(&binding, query);
    let mut response = MaybeUninit::<HostContactPageResponse>::uninit();
    let status = unsafe { destack_host_ios_contact_list(runtime_id, query, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    unsafe { decode_contact_page_response(response, operation) }
}

/// Submit one iOS contact-search request.
fn submit_contact_search(
    runtime_id: u64,
    operation: &'static str,
    query_text: &str,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let query = encode_contact_query(&binding, query);
    let mut response = MaybeUninit::<HostContactPageResponse>::uninit();
    let status = unsafe {
        destack_host_ios_contact_search(
            runtime_id,
            NativeStringRef::from(query_text),
            query,
            response.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    unsafe { decode_contact_page_response(response, operation) }
}

/// Submit one iOS contact-read request.
fn submit_contact_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<ContactValue> {
    let mut response = MaybeUninit::<HostContactResponse>::uninit();
    let status = unsafe {
        destack_host_ios_contact_read(runtime_id, NativeStringRef::from(id), response.as_mut_ptr())
    };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    unsafe { decode_contact_response(response, operation) }
}

/// Submit one iOS contact-create request.
fn submit_contact_create(
    runtime_id: u64,
    operation: &'static str,
    contact: &ContactDraftValue,
) -> RuntimeResult<String> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let draft = encode_contact_draft(&binding, contact);
    let mut response = MaybeUninit::<HostContactCreateResponse>::uninit();
    let status =
        unsafe { destack_host_ios_contact_create(runtime_id, draft, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    unsafe { decode_contact_create_response(response, operation) }
}

/// Submit one iOS contact-update request.
fn submit_contact_update(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
    contact: &ContactDraftValue,
) -> RuntimeResult<()> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let draft = encode_contact_draft(&binding, contact);
    let call_status =
        unsafe { destack_host_ios_contact_update(runtime_id, NativeStringRef::from(id), draft) };
    decode_callback_host_status(call_status, operation)
}
