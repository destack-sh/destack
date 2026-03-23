use super::callbacks::call_android_location_callback;
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::platform::os::{LocationSample, LocationWatchOptions};
use crate::runtime::NativeStringRef;

/// Read whether Android location services are enabled.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_location_services_enabled(
    runtime_id: u64,
    is_enabled: *mut bool,
) -> u32 {
    if is_enabled.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_location_callback(
        runtime_id,
        |callbacks| callbacks.services_enabled,
        |callback| unsafe { callback(runtime_id, is_enabled) },
    )
}

/// Read one Android last-known location sample.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_location_last_known(
    runtime_id: u64,
    sample: *mut LocationSample,
) -> u32 {
    if sample.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_location_callback(
        runtime_id,
        |callbacks| callbacks.last_known,
        |callback| unsafe { callback(runtime_id, sample) },
    )
}

/// Open one Android location watch.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_location_watch_open(
    runtime_id: u64,
    watch_id: NativeStringRef,
    options: LocationWatchOptions,
) -> u32 {
    call_android_location_callback(
        runtime_id,
        |callbacks| callbacks.watch_open,
        |callback| unsafe { callback(runtime_id, watch_id, options) },
    )
}

/// Close one Android location watch.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_location_watch_close(
    runtime_id: u64,
    watch_id: NativeStringRef,
) -> u32 {
    call_android_location_callback(
        runtime_id,
        |callbacks| callbacks.watch_close,
        |callback| unsafe { callback(runtime_id, watch_id) },
    )
}
