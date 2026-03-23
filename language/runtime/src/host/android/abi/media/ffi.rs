use super::callbacks::call_android_media_callback;
use crate::host::abi::media::{HostMediaAssetDescriptor, HostMediaPage, HostMediaQuery};
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// List Android media assets through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_list(
    runtime_id: u64,
    query: HostMediaQuery,
    output_page: *mut HostMediaPage,
) -> u32 {
    if output_page.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, query, output_page) },
    )
}

/// Describe one Android media asset through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_describe(
    runtime_id: u64,
    id: NativeStringRef,
    output_descriptor: *mut HostMediaAssetDescriptor,
) -> u32 {
    if output_descriptor.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.describe,
        |callback| unsafe { callback(runtime_id, id, output_descriptor) },
    )
}

/// Import one path into the Android media library through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_import_path(
    runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: *mut NativeStringRef,
) -> u32 {
    if output_id.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_media_callback(
        runtime_id,
        |callbacks| callbacks.import_path,
        |callback| unsafe { callback(runtime_id, path, kind, output_id) },
    )
}

/// Delete Android media assets through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_media_delete(
    runtime_id: u64,
    ids: NativeStringSlice,
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
