use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for listing Android calendars.
pub(crate) type AndroidHostCalendarListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for listing Android calendar events.
pub(crate) type AndroidHostCalendarEventListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for reading one Android calendar event.
pub(crate) type AndroidHostCalendarEventReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for creating one Android calendar event.
pub(crate) type AndroidHostCalendarEventCreateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for updating one Android calendar event.
pub(crate) type AndroidHostCalendarEventUpdateCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>, payload: NativeSlice<u8>) -> u32;

/// Host callback for deleting one Android calendar event.
pub(crate) type AndroidHostCalendarEventDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Callback table for Android host calendar request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostCalendarCallbacks {
    /// Callback for `calendarList`.
    pub list: Option<AndroidHostCalendarListCallback>,
    /// Callback for `calendarEventList`.
    pub event_list: Option<AndroidHostCalendarEventListCallback>,
    /// Callback for `calendarEventRead`.
    pub event_read: Option<AndroidHostCalendarEventReadCallback>,
    /// Callback for `calendarEventCreate`.
    pub event_create: Option<AndroidHostCalendarEventCreateCallback>,
    /// Callback for `calendarEventUpdate`.
    pub event_update: Option<AndroidHostCalendarEventUpdateCallback>,
    /// Callback for `calendarEventDelete`.
    pub event_delete: Option<AndroidHostCalendarEventDeleteCallback>,
}

/// Resolve and invoke one Android host calendar callback.
pub(super) fn call_android_calendar_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostCalendarCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.calendar), invoke)
}
