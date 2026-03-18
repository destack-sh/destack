use super::call_ios_contact_callback;
use crate::runtime::NativeSlice;

/// List iOS contacts through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_contact_list(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_ios_contact_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, payload, output, output_written) },
    )
}

/// Search iOS contacts through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_contact_search(
    runtime_id: u64,
    query_text: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_ios_contact_callback(
        runtime_id,
        |callbacks| callbacks.search,
        |callback| unsafe { callback(runtime_id, query_text, payload, output, output_written) },
    )
}

/// Read one iOS contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_contact_read(
    runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_ios_contact_callback(
        runtime_id,
        |callbacks| callbacks.read,
        |callback| unsafe { callback(runtime_id, id, output, output_written) },
    )
}

/// Create one iOS contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_contact_create(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_ios_contact_callback(
        runtime_id,
        |callbacks| callbacks.create,
        |callback| unsafe { callback(runtime_id, payload, output_id, output_written) },
    )
}

/// Update one iOS contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_contact_update(
    runtime_id: u64,
    id: NativeSlice<u8>,
    payload: NativeSlice<u8>,
) -> u32 {
    call_ios_contact_callback(
        runtime_id,
        |callbacks| callbacks.update,
        |callback| unsafe { callback(runtime_id, id, payload) },
    )
}

/// Delete one iOS contact through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_contact_delete(
    runtime_id: u64,
    id: NativeSlice<u8>,
) -> u32 {
    call_ios_contact_callback(
        runtime_id,
        |callbacks| callbacks.delete,
        |callback| unsafe { callback(runtime_id, id) },
    )
}
