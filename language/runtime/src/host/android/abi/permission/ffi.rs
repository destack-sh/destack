use super::callbacks::call_android_permission_callback;
use crate::host::abi::permission::HostPermissionRequest;

/// Open the Android permission settings surface.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_permission_open_settings(
    runtime_id: u64,
) -> u32 {
    call_android_permission_callback(
        runtime_id,
        |callbacks| callbacks.open_settings,
        |callback| unsafe { callback(runtime_id) },
    )
}

/// Open one Android permission request.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_permission_request(
    runtime_id: u64,
    request: HostPermissionRequest,
) -> u32 {
    call_android_permission_callback(
        runtime_id,
        |callbacks| callbacks.request,
        |callback| unsafe { callback(runtime_id, request) },
    )
}
