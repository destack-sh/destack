use super::call_android_notification_callback;
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::runtime::NativeSlice;

/// Request Android notification permission through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_request_permission(
    runtime_id: u64,
    state: *mut i32,
) -> u32 {
    if state.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.request_permission,
        |callback| unsafe { callback(runtime_id, state) },
    )
}

/// Cancel one Android notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_cancel(
    runtime_id: u64,
    id: NativeSlice<u8>,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.cancel,
        |callback| unsafe { callback(runtime_id, id) },
    )
}

/// Cancel every Android notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_cancel_all(
    runtime_id: u64,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.cancel_all,
        |callback| unsafe { callback(runtime_id) },
    )
}

/// List Android notification categories through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_category_list(
    runtime_id: u64,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.category_list,
        |callback| unsafe { callback(runtime_id, output, output_written) },
    )
}

/// Register Android notification categories through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_category_set(
    runtime_id: u64,
    payload: NativeSlice<u8>,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.category_set,
        |callback| unsafe { callback(runtime_id, payload) },
    )
}

/// List Android pending notifications through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_pending_list(
    runtime_id: u64,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.pending_list,
        |callback| unsafe { callback(runtime_id, output, output_written) },
    )
}

/// Cancel one Android pending notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_pending_cancel(
    runtime_id: u64,
    id: NativeSlice<u8>,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.pending_cancel,
        |callback| unsafe { callback(runtime_id, id) },
    )
}

/// Cancel every Android pending notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_pending_cancel_all(
    runtime_id: u64,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.pending_cancel_all,
        |callback| unsafe { callback(runtime_id) },
    )
}

/// Post one Android notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_post(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.post,
        |callback| unsafe { callback(runtime_id, payload, output_id, output_written) },
    )
}

/// Schedule one Android notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notification_schedule(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_notification_callback(
        runtime_id,
        |callbacks| callbacks.schedule,
        |callback| unsafe { callback(runtime_id, payload, output_id, output_written) },
    )
}
