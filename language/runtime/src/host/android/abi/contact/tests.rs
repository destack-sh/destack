use crate::host::android::abi::bindings::AndroidHostBindings;
use crate::host::android::abi::contact::callbacks::AndroidHostContactCallbacks;
use crate::host::android::abi::contact::ffi::{
    destack_host_android_contact_create, destack_host_android_contact_delete,
    destack_host_android_contact_list, destack_host_android_contact_read,
    destack_host_android_contact_search, destack_host_android_contact_update,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings, register_android_runtime,
};
use crate::host::core::registry::HostSessionRegistry;
use crate::host::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactDraftValue, ContactEmailValue, ContactNameValue,
    ContactOrganizationValue, ContactPageValue, ContactPhoneValue, ContactQueryValue, ContactValue,
};
use crate::runtime::NativeSlice;

unsafe extern "C" fn test_contact_list_callback(
    _runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let payload = unsafe { payload.as_slice() }.unwrap();
    let query = serde_json::from_slice::<ContactQueryValue>(payload).unwrap();

    assert_eq!(query, test_contact_query());

    write_json_output(output, output_written, &test_contact_page())
}

unsafe extern "C" fn test_contact_search_callback(
    _runtime_id: u64,
    query_text: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let query_text = unsafe { query_text.as_slice() }.unwrap();
    let payload = unsafe { payload.as_slice() }.unwrap();
    let query = serde_json::from_slice::<ContactQueryValue>(payload).unwrap();

    assert_eq!(std::str::from_utf8(query_text).unwrap(), "Ada");
    assert_eq!(query, test_contact_query());

    write_json_output(output, output_written, &test_contact_page())
}

unsafe extern "C" fn test_contact_read_callback(
    _runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let id = unsafe { id.as_slice() }.unwrap();
    assert_eq!(std::str::from_utf8(id).unwrap(), "contact-1");

    write_json_output(output, output_written, &test_contact())
}

unsafe extern "C" fn test_contact_create_callback(
    _runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let payload = unsafe { payload.as_slice() }.unwrap();
    let draft = serde_json::from_slice::<ContactDraftValue>(payload).unwrap();

    assert_eq!(draft, test_contact_draft());

    write_string_output(output_id, output_written, "contact-1")
}

unsafe extern "C" fn test_contact_update_callback(
    _runtime_id: u64,
    id: NativeSlice<u8>,
    payload: NativeSlice<u8>,
) -> u32 {
    let id = unsafe { id.as_slice() }.unwrap();
    let payload = unsafe { payload.as_slice() }.unwrap();
    let draft = serde_json::from_slice::<ContactDraftValue>(payload).unwrap();

    assert_eq!(std::str::from_utf8(id).unwrap(), "contact-1");
    assert_eq!(draft, test_contact_draft());

    HOST_STATUS_OK
}

unsafe extern "C" fn test_contact_delete_callback(_runtime_id: u64, id: NativeSlice<u8>) -> u32 {
    let id = unsafe { id.as_slice() }.unwrap();
    assert_eq!(std::str::from_utf8(id).unwrap(), "contact-1");

    HOST_STATUS_OK
}

#[test]
fn test_contact_callbacks_report_missing_runtime_registration() {
    let _lock = callback_test_lock().lock().unwrap();
    let runtime_id = HostSessionRegistry::allocate_session_id().0;
    let query = serde_json::to_vec(&test_contact_query()).unwrap();
    let status = unsafe {
        destack_host_android_contact_list(
            runtime_id,
            NativeSlice {
                data: query.as_ptr() as *mut u8,
                len: query.len() as u32,
            },
            empty_output(),
            std::ptr::null_mut(),
        )
    };

    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_contact_callbacks_report_unsupported_without_registered_handler() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(runtime_id, AndroidHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let query = serde_json::to_vec(&test_contact_query()).unwrap();
    let status = unsafe {
        destack_host_android_contact_list(
            runtime_id,
            NativeSlice {
                data: query.as_ptr() as *mut u8,
                len: query.len() as u32,
            },
            empty_output(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_contact_callbacks_route_registered_handlers() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let status = register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            contact: AndroidHostContactCallbacks {
                list: Some(test_contact_list_callback),
                search: Some(test_contact_search_callback),
                read: Some(test_contact_read_callback),
                create: Some(test_contact_create_callback),
                update: Some(test_contact_update_callback),
                delete: Some(test_contact_delete_callback),
            },
            ..AndroidHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    // list and search
    let query = serde_json::to_vec(&test_contact_query()).unwrap();
    let mut list_output = vec![0_u8; 2048];
    let mut list_written = 0_u32;
    let list_status = unsafe {
        destack_host_android_contact_list(
            runtime_id,
            NativeSlice {
                data: query.as_ptr() as *mut u8,
                len: query.len() as u32,
            },
            NativeSlice {
                data: list_output.as_mut_ptr(),
                len: list_output.len() as u32,
            },
            &mut list_written,
        )
    };
    assert_eq!(list_status, HOST_STATUS_OK);
    let page =
        serde_json::from_slice::<ContactPageValue>(&list_output[..list_written as usize]).unwrap();
    assert_eq!(page, test_contact_page());

    let search_text = b"Ada";
    let mut search_output = vec![0_u8; 2048];
    let mut search_written = 0_u32;
    let search_status = unsafe {
        destack_host_android_contact_search(
            runtime_id,
            NativeSlice {
                data: search_text.as_ptr() as *mut u8,
                len: search_text.len() as u32,
            },
            NativeSlice {
                data: query.as_ptr() as *mut u8,
                len: query.len() as u32,
            },
            NativeSlice {
                data: search_output.as_mut_ptr(),
                len: search_output.len() as u32,
            },
            &mut search_written,
        )
    };
    assert_eq!(search_status, HOST_STATUS_OK);
    let search_page =
        serde_json::from_slice::<ContactPageValue>(&search_output[..search_written as usize])
            .unwrap();
    assert_eq!(search_page, test_contact_page());

    // read one contact
    let contact_id = b"contact-1";
    let mut read_output = vec![0_u8; 2048];
    let mut read_written = 0_u32;
    let read_status = unsafe {
        destack_host_android_contact_read(
            runtime_id,
            NativeSlice {
                data: contact_id.as_ptr() as *mut u8,
                len: contact_id.len() as u32,
            },
            NativeSlice {
                data: read_output.as_mut_ptr(),
                len: read_output.len() as u32,
            },
            &mut read_written,
        )
    };
    assert_eq!(read_status, HOST_STATUS_OK);
    let contact =
        serde_json::from_slice::<ContactValue>(&read_output[..read_written as usize]).unwrap();
    assert_eq!(contact, test_contact());

    // create one contact
    let draft = serde_json::to_vec(&test_contact_draft()).unwrap();
    let mut create_output = vec![0_u8; 128];
    let mut create_written = 0_u32;
    let create_status = unsafe {
        destack_host_android_contact_create(
            runtime_id,
            NativeSlice {
                data: draft.as_ptr() as *mut u8,
                len: draft.len() as u32,
            },
            NativeSlice {
                data: create_output.as_mut_ptr(),
                len: create_output.len() as u32,
            },
            &mut create_written,
        )
    };
    assert_eq!(create_status, HOST_STATUS_OK);
    assert_eq!(
        std::str::from_utf8(&create_output[..create_written as usize]).unwrap(),
        "contact-1"
    );

    // update and delete
    let update_status = unsafe {
        destack_host_android_contact_update(
            runtime_id,
            NativeSlice {
                data: contact_id.as_ptr() as *mut u8,
                len: contact_id.len() as u32,
            },
            NativeSlice {
                data: draft.as_ptr() as *mut u8,
                len: draft.len() as u32,
            },
        )
    };
    assert_eq!(update_status, HOST_STATUS_OK);

    let delete_status = unsafe {
        destack_host_android_contact_delete(
            runtime_id,
            NativeSlice {
                data: contact_id.as_ptr() as *mut u8,
                len: contact_id.len() as u32,
            },
        )
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
}

fn empty_output() -> NativeSlice<u8> {
    NativeSlice {
        data: std::ptr::null_mut(),
        len: 0,
    }
}

fn write_json_output<T: serde::Serialize>(
    output: NativeSlice<u8>,
    output_written: *mut u32,
    value: &T,
) -> u32 {
    let bytes = serde_json::to_vec(value).unwrap();
    write_bytes_output(output, output_written, &bytes)
}

fn write_string_output(output: NativeSlice<u8>, output_written: *mut u32, value: &str) -> u32 {
    write_bytes_output(output, output_written, value.as_bytes())
}

fn write_bytes_output(output: NativeSlice<u8>, output_written: *mut u32, bytes: &[u8]) -> u32 {
    let output_written = unsafe { &mut *output_written };

    if output.len < bytes.len() as u32 {
        *output_written = bytes.len() as u32;
        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    let output = unsafe { output.as_mut_slice() }.unwrap();
    output[..bytes.len()].copy_from_slice(bytes);
    *output_written = bytes.len() as u32;

    HOST_STATUS_OK
}

fn test_contact_query() -> ContactQueryValue {
    ContactQueryValue {
        cursor: None,
        limit: Some(10),
        include_phones: true,
        include_emails: true,
        include_addresses: false,
        include_organization: true,
        include_notes: false,
    }
}

fn test_contact_page() -> ContactPageValue {
    ContactPageValue {
        contacts: vec![test_contact()],
        next_cursor: "next".to_string(),
        has_more: true,
    }
}

fn test_contact() -> ContactValue {
    ContactValue {
        id: "contact-1".to_string(),
        name: test_contact_name(),
        phones: vec![ContactPhoneValue {
            label: "mobile".to_string(),
            number: "+41790000000".to_string(),
            normalized_number: "+41790000000".to_string(),
            primary: true,
        }],
        emails: vec![ContactEmailValue {
            label: "work".to_string(),
            address: "ada@example.com".to_string(),
            primary: true,
        }],
        addresses: vec![ContactAddressValue {
            label: "home".to_string(),
            street: "Main Street 1".to_string(),
            city: "Zurich".to_string(),
            region: "ZH".to_string(),
            postal_code: "8000".to_string(),
            country: "Switzerland".to_string(),
            country_code: "CH".to_string(),
        }],
        organization: ContactOrganizationValue {
            company: "Destack".to_string(),
            department: "Runtime".to_string(),
            title: "Engineer".to_string(),
        },
        note: "friend".to_string(),
    }
}

fn test_contact_draft() -> ContactDraftValue {
    ContactDraftValue {
        name: test_contact_name(),
        phones: vec![ContactPhoneValue {
            label: "mobile".to_string(),
            number: "+41790000000".to_string(),
            normalized_number: "+41790000000".to_string(),
            primary: true,
        }],
        emails: vec![ContactEmailValue {
            label: "work".to_string(),
            address: "ada@example.com".to_string(),
            primary: true,
        }],
        addresses: vec![ContactAddressValue {
            label: "home".to_string(),
            street: "Main Street 1".to_string(),
            city: "Zurich".to_string(),
            region: "ZH".to_string(),
            postal_code: "8000".to_string(),
            country: "Switzerland".to_string(),
            country_code: "CH".to_string(),
        }],
        organization: ContactOrganizationValue {
            company: "Destack".to_string(),
            department: "Runtime".to_string(),
            title: "Engineer".to_string(),
        },
        note: "friend".to_string(),
    }
}

fn test_contact_name() -> ContactNameValue {
    ContactNameValue {
        given_name: "Ada".to_string(),
        middle_name: String::new(),
        family_name: "Lovelace".to_string(),
        prefix: String::new(),
        suffix: String::new(),
        nickname: "Ada".to_string(),
        phonetic_given_name: String::new(),
        phonetic_family_name: String::new(),
    }
}
