use crate::host::abi::background::{
    HostBackgroundStatus, HostBackgroundTaskDescriptor, HostBackgroundTaskOptions,
    HostBackgroundTaskResult,
};
use crate::host::android::abi::bindings::invoke_android_binding_callback;
use crate::platform::NativeArray;
use crate::runtime::NativeStringRef;

/// Host callback for reading Android background scheduler status.
pub(crate) type AndroidHostBackgroundStatusCallback =
    unsafe extern "C" fn(runtime_id: u64, status: *mut HostBackgroundStatus) -> u32;

/// Host callback for listing Android background task registrations.
pub(crate) type AndroidHostBackgroundListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    output_descriptors: *mut NativeArray<HostBackgroundTaskDescriptor>,
) -> u32;

/// Host callback for registering one Android background task.
pub(crate) type AndroidHostBackgroundRegisterCallback =
    unsafe extern "C" fn(runtime_id: u64, options: HostBackgroundTaskOptions) -> u32;

/// Host callback for unregistering one Android background task.
pub(crate) type AndroidHostBackgroundUnregisterCallback =
    unsafe extern "C" fn(runtime_id: u64, identifier: NativeStringRef) -> u32;

/// Host callback for triggering one Android background task in tests.
pub(crate) type AndroidHostBackgroundTriggerTestCallback = unsafe extern "C" fn(
    runtime_id: u64,
    identifier: NativeStringRef,
    is_triggered: *mut bool,
) -> u32;

/// Host callback for completing one Android background task execution.
pub(crate) type AndroidHostBackgroundCompleteCallback = unsafe extern "C" fn(
    runtime_id: u64,
    execution_id: NativeStringRef,
    result: HostBackgroundTaskResult,
) -> u32;

/// Callback table for Android host background request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBackgroundCallbacks {
    /// Callback for `backgroundStatus`.
    pub status: Option<AndroidHostBackgroundStatusCallback>,
    /// Callback for `backgroundList`.
    pub list: Option<AndroidHostBackgroundListCallback>,
    /// Callback for `backgroundRegister`.
    pub register: Option<AndroidHostBackgroundRegisterCallback>,
    /// Callback for `backgroundUnregister`.
    pub unregister: Option<AndroidHostBackgroundUnregisterCallback>,
    /// Callback for `backgroundTriggerTest`.
    pub trigger_test: Option<AndroidHostBackgroundTriggerTestCallback>,
    /// Callback for `backgroundComplete`.
    pub complete: Option<AndroidHostBackgroundCompleteCallback>,
}

/// Resolve and invoke one Android host background callback.
pub(super) fn call_android_background_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBackgroundCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.background), invoke)
}
