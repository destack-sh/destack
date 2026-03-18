use crate::host::ios::bridge::bindings::invoke_ios_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for listing iOS calendars.
pub(crate) type IosHostCalendarListCallback =
    unsafe extern "C" fn(runtime_id: u64, output: NativeSlice<u8>, output_written: *mut u32) -> u32;

/// Host callback for listing iOS calendar events.
pub(crate) type IosHostCalendarEventListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for reading one iOS calendar event.
pub(crate) type IosHostCalendarEventReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for creating one iOS calendar event.
pub(crate) type IosHostCalendarEventCreateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for updating one iOS calendar event.
pub(crate) type IosHostCalendarEventUpdateCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>, payload: NativeSlice<u8>) -> u32;

/// Host callback for deleting one iOS calendar event.
pub(crate) type IosHostCalendarEventDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Callback table for iOS host calendar request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostCalendarCallbacks {
    /// Callback for `calendarList`.
    pub list: Option<IosHostCalendarListCallback>,
    /// Callback for `calendarEventList`.
    pub event_list: Option<IosHostCalendarEventListCallback>,
    /// Callback for `calendarEventRead`.
    pub event_read: Option<IosHostCalendarEventReadCallback>,
    /// Callback for `calendarEventCreate`.
    pub event_create: Option<IosHostCalendarEventCreateCallback>,
    /// Callback for `calendarEventUpdate`.
    pub event_update: Option<IosHostCalendarEventUpdateCallback>,
    /// Callback for `calendarEventDelete`.
    pub event_delete: Option<IosHostCalendarEventDeleteCallback>,
}

/// Resolve and invoke one iOS host calendar callback.
pub(super) fn call_ios_calendar_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostCalendarCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.calendar), invoke)
}
