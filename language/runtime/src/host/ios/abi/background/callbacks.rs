use crate::host::ios::abi::bindings::invoke_ios_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for reading iOS background scheduler status.
pub(crate) type IosHostBackgroundStatusCallback =
    unsafe extern "C" fn(runtime_id: u64, status: *mut i32) -> u32;

/// Host callback for listing iOS background task registrations.
pub(crate) type IosHostBackgroundListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for registering one iOS background task.
pub(crate) type IosHostBackgroundRegisterCallback =
    unsafe extern "C" fn(runtime_id: u64, payload: NativeSlice<u8>) -> u32;

/// Host callback for unregistering one iOS background task.
pub(crate) type IosHostBackgroundUnregisterCallback =
    unsafe extern "C" fn(runtime_id: u64, identifier: NativeSlice<u8>) -> u32;

/// Host callback for triggering one iOS background task in tests.
pub(crate) type IosHostBackgroundTriggerTestCallback = unsafe extern "C" fn(
    runtime_id: u64,
    identifier: NativeSlice<u8>,
    is_triggered: *mut bool,
) -> u32;

/// Host callback for completing one iOS background task execution.
pub(crate) type IosHostBackgroundCompleteCallback =
    unsafe extern "C" fn(runtime_id: u64, execution_id: NativeSlice<u8>, result: i32) -> u32;

/// Callback table for iOS host background request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostBackgroundCallbacks {
    /// Callback for `backgroundStatus`.
    pub status: Option<IosHostBackgroundStatusCallback>,
    /// Callback for `backgroundList`.
    pub list: Option<IosHostBackgroundListCallback>,
    /// Callback for `backgroundRegister`.
    pub register: Option<IosHostBackgroundRegisterCallback>,
    /// Callback for `backgroundUnregister`.
    pub unregister: Option<IosHostBackgroundUnregisterCallback>,
    /// Callback for `backgroundTriggerTest`.
    pub trigger_test: Option<IosHostBackgroundTriggerTestCallback>,
    /// Callback for `backgroundComplete`.
    pub complete: Option<IosHostBackgroundCompleteCallback>,
}

/// Resolve and invoke one iOS host background callback.
pub(super) fn call_ios_background_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostBackgroundCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.background), invoke)
}
