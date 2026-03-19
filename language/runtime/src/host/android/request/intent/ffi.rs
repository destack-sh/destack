use super::call_android_intent_callback;
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Query whether the Android host can open one URL.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_intent_can_open_url(
    runtime_id: u64,
    url: NativeStringRef,
    is_supported: *mut bool,
) -> u32 {
    // require one writable output pointer
    if is_supported.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    call_android_intent_callback(
        runtime_id,
        |callbacks| callbacks.can_open_url,
        |callback| unsafe { callback(runtime_id, url, is_supported) },
    )
}

/// Open one URL through the Android host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_intent_open_url(
    runtime_id: u64,
    url: NativeStringRef,
) -> u32 {
    call_android_intent_callback(
        runtime_id,
        |callbacks| callbacks.open_url,
        |callback| unsafe { callback(runtime_id, url) },
    )
}

/// Open one path through the Android host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_intent_open_path(
    runtime_id: u64,
    path: NativeStringRef,
) -> u32 {
    call_android_intent_callback(
        runtime_id,
        |callbacks| callbacks.open_path,
        |callback| unsafe { callback(runtime_id, path) },
    )
}

/// Share one text payload through the Android host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_intent_share_text(
    runtime_id: u64,
    text: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32 {
    call_android_intent_callback(
        runtime_id,
        |callbacks| callbacks.share_text,
        |callback| unsafe { callback(runtime_id, text, has_mime_type, mime_type) },
    )
}

/// Share one path list through the Android host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_intent_share_paths(
    runtime_id: u64,
    paths: NativeStringSlice,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32 {
    call_android_intent_callback(
        runtime_id,
        |callbacks| callbacks.share_paths,
        |callback| unsafe { callback(runtime_id, paths, has_mime_type, mime_type) },
    )
}
