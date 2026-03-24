use crate::host::abi::notification::HostNotificationRequest;
use crate::host::ios::abi::bindings::invoke_ios_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for canceling one iOS notification.
pub(crate) type IosHostNotificationCancelCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Host callback for canceling every iOS notification.
pub(crate) type IosHostNotificationCancelAllCallback = unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for posting one iOS notification.
pub(crate) type IosHostNotificationPostCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostNotificationRequest) -> u32;

/// Callback table for iOS host notification request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostNotificationCallbacks {
    /// Callback for `notificationCancel`.
    pub cancel: Option<IosHostNotificationCancelCallback>,
    /// Callback for `notificationCancelAll`.
    pub cancel_all: Option<IosHostNotificationCancelAllCallback>,
    /// Callback for `notificationPost`.
    pub post: Option<IosHostNotificationPostCallback>,
}

/// Resolve and invoke one iOS host notification callback.
pub(super) fn call_ios_notification_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostNotificationCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(
        runtime_id,
        |bindings| resolve(&bindings.notification),
        invoke,
    )
}
