use crate::diagnostic::RuntimeResult;
use crate::host::core::callback::{
    decode_callback_host_status, encode_callback_host_json, read_buffered_callback_host_json,
    read_buffered_callback_host_string,
};
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::host::ios::abi::contact::{
    destack_host_ios_contact_create, destack_host_ios_contact_delete,
    destack_host_ios_contact_list, destack_host_ios_contact_read, destack_host_ios_contact_search,
    destack_host_ios_contact_update,
};
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::runtime::NativeSlice;

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
            let id = id.as_bytes();
            let call_status = unsafe {
                destack_host_ios_contact_delete(
                    runtime_id,
                    NativeSlice {
                        data: id.as_ptr() as *mut u8,
                        len: id.len() as u32,
                    },
                )
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
    let payload = encode_callback_host_json(query, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };

    read_buffered_callback_host_json(operation, "contact page", |output, output_written| unsafe {
        destack_host_ios_contact_list(runtime_id, payload, output, output_written)
    })
}

/// Submit one iOS contact-search request.
fn submit_contact_search(
    runtime_id: u64,
    operation: &'static str,
    query_text: &str,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let query_text = query_text.as_bytes();
    let query_text = NativeSlice {
        data: query_text.as_ptr() as *mut u8,
        len: query_text.len() as u32,
    };
    let payload = encode_callback_host_json(query, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };

    read_buffered_callback_host_json(operation, "contact page", |output, output_written| unsafe {
        destack_host_ios_contact_search(runtime_id, query_text, payload, output, output_written)
    })
}

/// Submit one iOS contact-read request.
fn submit_contact_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<ContactValue> {
    let id = id.as_bytes();
    let id = NativeSlice {
        data: id.as_ptr() as *mut u8,
        len: id.len() as u32,
    };

    read_buffered_callback_host_json(operation, "contact", |output, output_written| unsafe {
        destack_host_ios_contact_read(runtime_id, id, output, output_written)
    })
}

/// Submit one iOS contact-create request.
fn submit_contact_create(
    runtime_id: u64,
    operation: &'static str,
    contact: &ContactDraftValue,
) -> RuntimeResult<String> {
    let payload = encode_callback_host_json(contact, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };

    read_buffered_callback_host_string(operation, |output_id, output_written| unsafe {
        destack_host_ios_contact_create(runtime_id, payload, output_id, output_written)
    })
}

/// Submit one iOS contact-update request.
fn submit_contact_update(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
    contact: &ContactDraftValue,
) -> RuntimeResult<()> {
    let id = id.as_bytes();
    let id = NativeSlice {
        data: id.as_ptr() as *mut u8,
        len: id.len() as u32,
    };
    let payload = encode_callback_host_json(contact, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };
    let call_status = unsafe { destack_host_ios_contact_update(runtime_id, id, payload) };
    decode_callback_host_status(call_status, operation)
}
