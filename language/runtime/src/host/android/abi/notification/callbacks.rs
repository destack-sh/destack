use crate::host::abi::notification::HostNotificationRequest;
use crate::host::android::abi::bindings::invoke_android_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for canceling one Android notification.
pub(crate) type AndroidHostNotificationCancelCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Host callback for canceling every Android notification.
pub(crate) type AndroidHostNotificationCancelAllCallback =
    unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for posting one Android notification.
pub(crate) type AndroidHostNotificationPostCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostNotificationRequest) -> u32;

/// Callback table for Android host notification request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostNotificationCallbacks {
    /// Callback for `notificationCancel`.
    pub cancel: Option<AndroidHostNotificationCancelCallback>,
    /// Callback for `notificationCancelAll`.
    pub cancel_all: Option<AndroidHostNotificationCancelAllCallback>,
    /// Callback for `notificationPost`.
    pub post: Option<AndroidHostNotificationPostCallback>,
}

/// Resolve and invoke one Android host notification callback.
pub(super) fn call_android_notification_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostNotificationCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(
        runtime_id,
        |bindings| resolve(&bindings.notification),
        invoke,
    )
}
