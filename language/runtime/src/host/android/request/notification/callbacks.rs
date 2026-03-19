use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for requesting Android notification permission.
pub(crate) type AndroidHostNotificationRequestPermissionCallback =
    unsafe extern "C" fn(runtime_id: u64, state: *mut i32) -> u32;

/// Host callback for canceling one Android notification.
pub(crate) type AndroidHostNotificationCancelCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Host callback for canceling every Android notification.
pub(crate) type AndroidHostNotificationCancelAllCallback =
    unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for listing Android notification categories.
pub(crate) type AndroidHostNotificationCategoryListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for registering Android notification categories.
pub(crate) type AndroidHostNotificationCategorySetCallback =
    unsafe extern "C" fn(runtime_id: u64, payload: NativeSlice<u8>) -> u32;

/// Host callback for listing Android pending notifications.
pub(crate) type AndroidHostNotificationPendingListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for canceling one Android pending notification.
pub(crate) type AndroidHostNotificationPendingCancelCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Host callback for canceling every Android pending notification.
pub(crate) type AndroidHostNotificationPendingCancelAllCallback =
    unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for posting one Android notification.
pub(crate) type AndroidHostNotificationPostCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for scheduling one Android notification.
pub(crate) type AndroidHostNotificationScheduleCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Callback table for Android host notification request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostNotificationCallbacks {
    /// Callback for `notificationRequestPermission`.
    pub request_permission: Option<AndroidHostNotificationRequestPermissionCallback>,
    /// Callback for `notificationCancel`.
    pub cancel: Option<AndroidHostNotificationCancelCallback>,
    /// Callback for `notificationCancelAll`.
    pub cancel_all: Option<AndroidHostNotificationCancelAllCallback>,
    /// Callback for `notificationCategoryList`.
    pub category_list: Option<AndroidHostNotificationCategoryListCallback>,
    /// Callback for `notificationCategorySet`.
    pub category_set: Option<AndroidHostNotificationCategorySetCallback>,
    /// Callback for `notificationPendingList`.
    pub pending_list: Option<AndroidHostNotificationPendingListCallback>,
    /// Callback for `notificationPendingCancel`.
    pub pending_cancel: Option<AndroidHostNotificationPendingCancelCallback>,
    /// Callback for `notificationPendingCancelAll`.
    pub pending_cancel_all: Option<AndroidHostNotificationPendingCancelAllCallback>,
    /// Callback for `notificationPost`.
    pub post: Option<AndroidHostNotificationPostCallback>,
    /// Callback for `notificationSchedule`.
    pub schedule: Option<AndroidHostNotificationScheduleCallback>,
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
