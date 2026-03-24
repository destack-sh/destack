use super::callbacks::call_ios_notification_callback;
use crate::host::abi::notification::HostNotificationRequest;
use crate::runtime::NativeSlice;

/// Cancel one iOS notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notification_cancel(
    runtime_id: u64,
    id: NativeSlice<u8>,
) -> u32 {
    call_ios_notification_callback(
        runtime_id,
        |callbacks| callbacks.cancel,
        |callback| unsafe { callback(runtime_id, id) },
    )
}

/// Cancel every iOS notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notification_cancel_all(runtime_id: u64) -> u32 {
    call_ios_notification_callback(
        runtime_id,
        |callbacks| callbacks.cancel_all,
        |callback| unsafe { callback(runtime_id) },
    )
}

/// Post one iOS notification through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notification_post(
    runtime_id: u64,
    request: HostNotificationRequest,
) -> u32 {
    call_ios_notification_callback(
        runtime_id,
        |callbacks| callbacks.post,
        |callback| unsafe { callback(runtime_id, request) },
    )
}
