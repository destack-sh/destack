use super::call_android_media_callback;
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::runtime::{NativeSlice, NativeStringRef};

/// List Android media assets through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_list(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, payload, output, output_written) },
    )
}

/// Describe one Android media asset through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_describe(
    runtime_id: u64,
    id: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.describe,
        |callback| unsafe { callback(runtime_id, id, output, output_written) },
    )
}

/// Import one path into the Android media library through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_import_path(
    runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.import_path,
        |callback| unsafe { callback(runtime_id, path, kind, output_id, output_written) },
    )
}

/// Delete Android media assets through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_delete(
    runtime_id: u64,
    ids: NativeSlice<u8>,
    deleted_count: *mut u32,
) -> u32 {
    if deleted_count.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.delete,
        |callback| unsafe { callback(runtime_id, ids, deleted_count) },
    )
}
