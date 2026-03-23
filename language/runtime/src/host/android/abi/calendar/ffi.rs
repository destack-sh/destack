use super::callbacks::call_android_calendar_callback;
use crate::runtime::NativeSlice;

/// List Android calendars through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_list(
    runtime_id: u64,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, output, output_written) },
    )
}

/// List Android calendar events through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_list(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_list,
        |callback| unsafe { callback(runtime_id, payload, output, output_written) },
    )
}

/// Read one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_read(
    runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_read,
        |callback| unsafe { callback(runtime_id, id, output, output_written) },
    )
}

/// Create one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_create(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_create,
        |callback| unsafe { callback(runtime_id, payload, output_id, output_written) },
    )
}

/// Update one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_update(
    runtime_id: u64,
    id: NativeSlice<u8>,
    payload: NativeSlice<u8>,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_update,
        |callback| unsafe { callback(runtime_id, id, payload) },
    )
}

/// Delete one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_delete(
    runtime_id: u64,
    id: NativeSlice<u8>,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_delete,
        |callback| unsafe { callback(runtime_id, id) },
    )
}
