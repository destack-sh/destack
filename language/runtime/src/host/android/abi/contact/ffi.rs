use super::callbacks::call_android_contact_callback;
use crate::host::abi::contact::HostContactQuery;
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::platform::os::abi_generated::{Contact, ContactDraft, ContactPage};
use crate::runtime::NativeStringRef;

/// List Android contacts through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_contact_list(
    runtime_id: u64,
    query: HostContactQuery,
    output_page: *mut ContactPage,
) -> u32 {
    if output_page.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_contact_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, query, output_page) },
    )
}

/// Search Android contacts through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_contact_search(
    runtime_id: u64,
    query_text: NativeStringRef,
    query: HostContactQuery,
    output_page: *mut ContactPage,
) -> u32 {
    if output_page.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_contact_callback(
        runtime_id,
        |callbacks| callbacks.search,
        |callback| unsafe { callback(runtime_id, query_text, query, output_page) },
    )
}

/// Read one Android contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_contact_read(
    runtime_id: u64,
    id: NativeStringRef,
    output_contact: *mut Contact,
) -> u32 {
    if output_contact.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_contact_callback(
        runtime_id,
        |callbacks| callbacks.read,
        |callback| unsafe { callback(runtime_id, id, output_contact) },
    )
}

/// Create one Android contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_contact_create(
    runtime_id: u64,
    draft: ContactDraft,
    output_id: *mut NativeStringRef,
) -> u32 {
    if output_id.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_contact_callback(
        runtime_id,
        |callbacks| callbacks.create,
        |callback| unsafe { callback(runtime_id, draft, output_id) },
    )
}

/// Update one Android contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_contact_update(
    runtime_id: u64,
    id: NativeStringRef,
    draft: ContactDraft,
) -> u32 {
    call_android_contact_callback(
        runtime_id,
        |callbacks| callbacks.update,
        |callback| unsafe { callback(runtime_id, id, draft) },
    )
}

/// Delete one Android contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_contact_delete(
    runtime_id: u64,
    id: NativeStringRef,
) -> u32 {
    call_android_contact_callback(
        runtime_id,
        |callbacks| callbacks.delete,
        |callback| unsafe { callback(runtime_id, id) },
    )
}
