use crate::host::ios::abi::bindings::invoke_ios_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for requesting iOS notification permission.
pub(crate) type IosHostNotificationRequestPermissionCallback =
    unsafe extern "C" fn(runtime_id: u64, state: *mut i32) -> u32;

/// Host callback for canceling one iOS notification.
pub(crate) type IosHostNotificationCancelCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Host callback for canceling every iOS notification.
pub(crate) type IosHostNotificationCancelAllCallback = unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for listing iOS notification categories.
pub(crate) type IosHostNotificationCategoryListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for registering iOS notification categories.
pub(crate) type IosHostNotificationCategorySetCallback =
    unsafe extern "C" fn(runtime_id: u64, payload: NativeSlice<u8>) -> u32;

/// Host callback for listing iOS pending notifications.
pub(crate) type IosHostNotificationPendingListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for canceling one iOS pending notification.
pub(crate) type IosHostNotificationPendingCancelCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Host callback for canceling every iOS pending notification.
pub(crate) type IosHostNotificationPendingCancelAllCallback =
    unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for posting one iOS notification.
pub(crate) type IosHostNotificationPostCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for scheduling one iOS notification.
pub(crate) type IosHostNotificationScheduleCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Callback table for iOS host notification request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostNotificationCallbacks {
    /// Callback for `notificationRequestPermission`.
    pub request_permission: Option<IosHostNotificationRequestPermissionCallback>,
    /// Callback for `notificationCancel`.
    pub cancel: Option<IosHostNotificationCancelCallback>,
    /// Callback for `notificationCancelAll`.
    pub cancel_all: Option<IosHostNotificationCancelAllCallback>,
    /// Callback for `notificationCategoryList`.
    pub category_list: Option<IosHostNotificationCategoryListCallback>,
    /// Callback for `notificationCategorySet`.
    pub category_set: Option<IosHostNotificationCategorySetCallback>,
    /// Callback for `notificationPendingList`.
    pub pending_list: Option<IosHostNotificationPendingListCallback>,
    /// Callback for `notificationPendingCancel`.
    pub pending_cancel: Option<IosHostNotificationPendingCancelCallback>,
    /// Callback for `notificationPendingCancelAll`.
    pub pending_cancel_all: Option<IosHostNotificationPendingCancelAllCallback>,
    /// Callback for `notificationPost`.
    pub post: Option<IosHostNotificationPostCallback>,
    /// Callback for `notificationSchedule`.
    pub schedule: Option<IosHostNotificationScheduleCallback>,
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
