use super::call_ios_background_callback;
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::runtime::NativeSlice;

/// Read iOS background scheduler status through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_background_status(
    runtime_id: u64,
    status: *mut i32,
) -> u32 {
    if status.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_ios_background_callback(
        runtime_id,
        |callbacks| callbacks.status,
        |callback| unsafe { callback(runtime_id, status) },
    )
}

/// List iOS background task registrations through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_background_list(
    runtime_id: u64,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_ios_background_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, output, output_written) },
    )
}

/// Register one iOS background task through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_background_register(
    runtime_id: u64,
    payload: NativeSlice<u8>,
) -> u32 {
    call_ios_background_callback(
        runtime_id,
        |callbacks| callbacks.register,
        |callback| unsafe { callback(runtime_id, payload) },
    )
}

/// Unregister one iOS background task through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_background_unregister(
    runtime_id: u64,
    identifier: NativeSlice<u8>,
) -> u32 {
    call_ios_background_callback(
        runtime_id,
        |callbacks| callbacks.unregister,
        |callback| unsafe { callback(runtime_id, identifier) },
    )
}

/// Trigger one iOS background task through the host test bridge.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_background_trigger_test(
    runtime_id: u64,
    identifier: NativeSlice<u8>,
    is_triggered: *mut bool,
) -> u32 {
    if is_triggered.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_ios_background_callback(
        runtime_id,
        |callbacks| callbacks.trigger_test,
        |callback| unsafe { callback(runtime_id, identifier, is_triggered) },
    )
}

/// Complete one iOS background task execution through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_background_complete(
    runtime_id: u64,
    execution_id: NativeSlice<u8>,
    result: i32,
) -> u32 {
    call_ios_background_callback(
        runtime_id,
        |callbacks| callbacks.complete,
        |callback| unsafe { callback(runtime_id, execution_id, result) },
    )
}
