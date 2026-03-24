use std::mem::MaybeUninit;
use std::sync::Arc;

use crate::host::abi::contact::HostContactQuery;
use crate::host::abi::core::{HostOptionalStringRef, HostOptionalU32};
use crate::host::core::registry::HostSessionRegistrationGuard;
use crate::host::core::{HostQueue, HostSessionRegistry};
use crate::host::ios::abi::bindings::{IosHostBindings, destack_host_ios_register_bindings};
use crate::host::ios::abi::contact::callbacks::IosHostContactCallbacks;
use crate::host::ios::abi::contact::ffi::{
    destack_host_ios_contact_create, destack_host_ios_contact_delete,
    destack_host_ios_contact_list, destack_host_ios_contact_read, destack_host_ios_contact_search,
    destack_host_ios_contact_update,
};
use crate::host::ios::abi::registry::unregister_ios_bindings;
use crate::host::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    Platform,
};
use crate::platform::os::abi_generated::{
    Contact, ContactDraft, ContactDraftValue, ContactName, ContactNameValue, ContactOrganization,
    ContactOrganizationValue, ContactPage, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::platform::{NativeAbiCodec, NativeArray};
use crate::runtime::NativeStringRef;

/// Register one temporary iOS host queue and keep registration state alive.
fn register_ios_runtime() -> (Arc<HostQueue>, HostSessionRegistrationGuard, u64) {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::clone(&queue),
        Some(unregister_ios_bindings),
    );
    let runtime_id = registration.host_session_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped iOS host bindings payload.
fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
}

unsafe extern "C" fn test_contact_list_callback(
    _runtime_id: u64,
    query: HostContactQuery,
    output_page: *mut ContactPage,
) -> u32 {
    let query = unsafe { query.into_value() }.unwrap();
    assert_eq!(query, test_contact_query());

    unsafe {
        *output_page = test_host_contact_page();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_contact_search_callback(
    _runtime_id: u64,
    query_text: NativeStringRef,
    query: HostContactQuery,
    output_page: *mut ContactPage,
) -> u32 {
    let query_text = unsafe { query_text.as_str() }.unwrap();
    let query = unsafe { query.into_value() }.unwrap();

    assert_eq!(query_text, "Ada");
    assert_eq!(query, test_contact_query());

    unsafe {
        *output_page = test_host_contact_page();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_contact_read_callback(
    _runtime_id: u64,
    id: NativeStringRef,
    output_contact: *mut Contact,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    assert_eq!(id, "contact-1");

    unsafe {
        *output_contact = test_host_contact();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_contact_create_callback(
    _runtime_id: u64,
    draft: ContactDraft,
    output_id: *mut NativeStringRef,
) -> u32 {
    let draft = unsafe { draft.into_value() }.unwrap();
    assert_eq!(draft, test_contact_draft());

    unsafe {
        *output_id = NativeStringRef::from("contact-1");
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_contact_update_callback(
    _runtime_id: u64,
    id: NativeStringRef,
    draft: ContactDraft,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    let draft = unsafe { draft.into_value() }.unwrap();

    assert_eq!(id, "contact-1");
    assert_eq!(draft, test_contact_draft());

    HOST_STATUS_OK
}

unsafe extern "C" fn test_contact_delete_callback(_runtime_id: u64, id: NativeStringRef) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    assert_eq!(id, "contact-1");

    HOST_STATUS_OK
}

#[test]
fn test_contact_callbacks_report_missing_runtime_registration() {
    let runtime_id = HostSessionRegistry::allocate_session_id().0;
    let query = test_host_contact_query();
    let mut output_page = MaybeUninit::<ContactPage>::uninit();
    let status =
        unsafe { destack_host_ios_contact_list(runtime_id, query, output_page.as_mut_ptr()) };

    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_contact_callbacks_report_unsupported_without_registered_handler() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(runtime_id, IosHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let query = test_host_contact_query();
    let mut output_page = MaybeUninit::<ContactPage>::uninit();
    let status =
        unsafe { destack_host_ios_contact_list(runtime_id, query, output_page.as_mut_ptr()) };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_contact_callbacks_route_registered_handlers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            contact: IosHostContactCallbacks {
                list: Some(test_contact_list_callback),
                search: Some(test_contact_search_callback),
                read: Some(test_contact_read_callback),
                create: Some(test_contact_create_callback),
                update: Some(test_contact_update_callback),
                delete: Some(test_contact_delete_callback),
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let mut list_output = MaybeUninit::<ContactPage>::uninit();
    let list_status = unsafe {
        destack_host_ios_contact_list(
            runtime_id,
            test_host_contact_query(),
            list_output.as_mut_ptr(),
        )
    };
    assert_eq!(list_status, HOST_STATUS_OK);
    let page = unsafe { list_output.assume_init().into_value() }.unwrap();
    assert_eq!(page, test_contact_page());

    let mut search_output = MaybeUninit::<ContactPage>::uninit();
    let search_status = unsafe {
        destack_host_ios_contact_search(
            runtime_id,
            NativeStringRef::from("Ada"),
            test_host_contact_query(),
            search_output.as_mut_ptr(),
        )
    };
    assert_eq!(search_status, HOST_STATUS_OK);
    let search_page = unsafe { search_output.assume_init().into_value() }.unwrap();
    assert_eq!(search_page, test_contact_page());

    let mut read_output = MaybeUninit::<Contact>::uninit();
    let read_status = unsafe {
        destack_host_ios_contact_read(
            runtime_id,
            NativeStringRef::from("contact-1"),
            read_output.as_mut_ptr(),
        )
    };
    assert_eq!(read_status, HOST_STATUS_OK);
    let contact = unsafe { read_output.assume_init().into_value() }.unwrap();
    assert_eq!(contact, test_contact());

    let mut create_output = MaybeUninit::<NativeStringRef>::uninit();
    let create_status = unsafe {
        destack_host_ios_contact_create(
            runtime_id,
            test_host_contact_draft(),
            create_output.as_mut_ptr(),
        )
    };
    assert_eq!(create_status, HOST_STATUS_OK);
    let create_output = unsafe { create_output.assume_init() };
    assert_eq!(unsafe { create_output.as_str() }.unwrap(), "contact-1");

    let update_status = unsafe {
        destack_host_ios_contact_update(
            runtime_id,
            NativeStringRef::from("contact-1"),
            test_host_contact_draft(),
        )
    };
    assert_eq!(update_status, HOST_STATUS_OK);

    let delete_status =
        unsafe { destack_host_ios_contact_delete(runtime_id, NativeStringRef::from("contact-1")) };
    assert_eq!(delete_status, HOST_STATUS_OK);
}

#[test]
fn test_contact_callbacks_require_output_pointers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            contact: IosHostContactCallbacks {
                list: Some(test_contact_list_callback),
                read: Some(test_contact_read_callback),
                create: Some(test_contact_create_callback),
                ..IosHostContactCallbacks::default()
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let list_status = unsafe {
        destack_host_ios_contact_list(runtime_id, test_host_contact_query(), std::ptr::null_mut())
    };
    assert_eq!(list_status, HOST_STATUS_INVALID_ARGUMENT);

    let read_status = unsafe {
        destack_host_ios_contact_read(
            runtime_id,
            NativeStringRef::from("contact-1"),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(read_status, HOST_STATUS_INVALID_ARGUMENT);

    let create_status = unsafe {
        destack_host_ios_contact_create(runtime_id, test_host_contact_draft(), std::ptr::null_mut())
    };
    assert_eq!(create_status, HOST_STATUS_INVALID_ARGUMENT);
}

fn empty_array<T>() -> NativeArray<T> {
    NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    }
}

fn leak_array<T>(values: Vec<T>) -> NativeArray<T> {
    let values = values.into_boxed_slice();
    let len = values.len() as u32;
    let data = Box::leak(values).as_mut_ptr();

    NativeArray {
        data,
        len,
        capacity: len,
    }
}

fn test_host_contact_query() -> HostContactQuery {
    HostContactQuery {
        cursor: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("cursor-1"),
        },
        limit: HostOptionalU32 {
            has_value: true,
            value: 10,
        },
        include_phones: true,
        include_emails: true,
        include_addresses: false,
        include_organization: true,
        include_notes: false,
    }
}

fn test_contact_query() -> ContactQueryValue {
    ContactQueryValue {
        cursor: Some("cursor-1".to_string()),
        limit: Some(10),
        include_phones: true,
        include_emails: true,
        include_addresses: false,
        include_organization: true,
        include_notes: false,
    }
}

fn test_host_contact_page() -> ContactPage {
    ContactPage {
        contacts: leak_array(vec![test_host_contact()]),
        next_cursor: NativeStringRef::from("next"),
        has_more: true,
    }
}

fn test_contact_page() -> ContactPageValue {
    ContactPageValue {
        contacts: vec![test_contact()],
        next_cursor: "next".to_string(),
        has_more: true,
    }
}

fn test_host_contact() -> Contact {
    Contact {
        id: NativeStringRef::from("contact-1"),
        name: ContactName {
            given_name: NativeStringRef::from("Ada"),
            middle_name: NativeStringRef::from(""),
            family_name: NativeStringRef::from("Lovelace"),
            prefix: NativeStringRef::from(""),
            suffix: NativeStringRef::from(""),
            nickname: NativeStringRef::from(""),
            phonetic_given_name: NativeStringRef::from(""),
            phonetic_family_name: NativeStringRef::from(""),
        },
        phones: empty_array(),
        emails: empty_array(),
        addresses: empty_array(),
        organization: ContactOrganization {
            company: NativeStringRef::from("Analytical Engine"),
            department: NativeStringRef::from(""),
            title: NativeStringRef::from("Programmer"),
        },
        note: NativeStringRef::from(""),
    }
}

fn test_contact() -> ContactValue {
    ContactValue {
        id: "contact-1".to_string(),
        name: ContactNameValue {
            given_name: "Ada".to_string(),
            middle_name: "".to_string(),
            family_name: "Lovelace".to_string(),
            prefix: "".to_string(),
            suffix: "".to_string(),
            nickname: "".to_string(),
            phonetic_given_name: "".to_string(),
            phonetic_family_name: "".to_string(),
        },
        phones: vec![],
        emails: vec![],
        addresses: vec![],
        organization: ContactOrganizationValue {
            company: "Analytical Engine".to_string(),
            department: "".to_string(),
            title: "Programmer".to_string(),
        },
        note: "".to_string(),
    }
}

fn test_host_contact_draft() -> ContactDraft {
    ContactDraft {
        name: ContactName {
            given_name: NativeStringRef::from("Ada"),
            middle_name: NativeStringRef::from(""),
            family_name: NativeStringRef::from("Lovelace"),
            prefix: NativeStringRef::from(""),
            suffix: NativeStringRef::from(""),
            nickname: NativeStringRef::from(""),
            phonetic_given_name: NativeStringRef::from(""),
            phonetic_family_name: NativeStringRef::from(""),
        },
        phones: empty_array(),
        emails: empty_array(),
        addresses: empty_array(),
        organization: ContactOrganization {
            company: NativeStringRef::from("Analytical Engine"),
            department: NativeStringRef::from(""),
            title: NativeStringRef::from("Programmer"),
        },
        note: NativeStringRef::from(""),
    }
}

fn test_contact_draft() -> ContactDraftValue {
    ContactDraftValue {
        name: ContactNameValue {
            given_name: "Ada".to_string(),
            middle_name: "".to_string(),
            family_name: "Lovelace".to_string(),
            prefix: "".to_string(),
            suffix: "".to_string(),
            nickname: "".to_string(),
            phonetic_given_name: "".to_string(),
            phonetic_family_name: "".to_string(),
        },
        phones: vec![],
        emails: vec![],
        addresses: vec![],
        organization: ContactOrganizationValue {
            company: "Analytical Engine".to_string(),
            department: "".to_string(),
            title: "Programmer".to_string(),
        },
        note: "".to_string(),
    }
}
