use super::callbacks::call_android_background_callback;
use crate::host::abi::background::{
    HostBackgroundStatus, HostBackgroundTaskDescriptor, HostBackgroundTaskOptions,
    HostBackgroundTaskResult,
};
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;
use crate::platform::NativeArray;
use crate::runtime::NativeStringRef;

/// Read Android background scheduler status through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_background_status(
    runtime_id: u64,
    status: *mut HostBackgroundStatus,
) -> u32 {
    if status.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_background_callback(
        runtime_id,
        |callbacks| callbacks.status,
        |callback| unsafe { callback(runtime_id, status) },
    )
}

/// List Android background task registrations through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_background_list(
    runtime_id: u64,
    output_descriptors: *mut NativeArray<HostBackgroundTaskDescriptor>,
) -> u32 {
    if output_descriptors.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_background_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, output_descriptors) },
    )
}

/// Register one Android background task through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_background_register(
    runtime_id: u64,
    options: HostBackgroundTaskOptions,
) -> u32 {
    call_android_background_callback(
        runtime_id,
        |callbacks| callbacks.register,
        |callback| unsafe { callback(runtime_id, options) },
    )
}

/// Unregister one Android background task through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_background_unregister(
    runtime_id: u64,
    identifier: NativeStringRef,
) -> u32 {
    call_android_background_callback(
        runtime_id,
        |callbacks| callbacks.unregister,
        |callback| unsafe { callback(runtime_id, identifier) },
    )
}

/// Trigger one Android background task through the host test bridge.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_background_trigger_test(
    runtime_id: u64,
    identifier: NativeStringRef,
    is_triggered: *mut bool,
) -> u32 {
    if is_triggered.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_background_callback(
        runtime_id,
        |callbacks| callbacks.trigger_test,
        |callback| unsafe { callback(runtime_id, identifier, is_triggered) },
    )
}

/// Complete one Android background task execution through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_background_complete(
    runtime_id: u64,
    execution_id: NativeStringRef,
    result: HostBackgroundTaskResult,
) -> u32 {
    call_android_background_callback(
        runtime_id,
        |callbacks| callbacks.complete,
        |callback| unsafe { callback(runtime_id, execution_id, result) },
    )
}
