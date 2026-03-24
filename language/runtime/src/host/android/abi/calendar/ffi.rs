use super::callbacks::call_android_calendar_callback;
use crate::host::abi::calendar::{
    HostCalendarDescriptorArray, HostCalendarEvent, HostCalendarEventArray, HostCalendarEventDraft,
    HostCalendarEventId, HostCalendarEventQuery,
};
use crate::host::core::HOST_STATUS_INVALID_ARGUMENT;

/// List Android calendars through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_list(
    runtime_id: u64,
    output_calendars: *mut HostCalendarDescriptorArray,
) -> u32 {
    if output_calendars.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.list,
        |callback| unsafe { callback(runtime_id, output_calendars) },
    )
}

/// List Android calendar events through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_list(
    runtime_id: u64,
    query: HostCalendarEventQuery,
    output_events: *mut HostCalendarEventArray,
) -> u32 {
    if output_events.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_list,
        |callback| unsafe { callback(runtime_id, query, output_events) },
    )
}

/// Read one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_read(
    runtime_id: u64,
    id: HostCalendarEventId,
    output_event: *mut HostCalendarEvent,
) -> u32 {
    if output_event.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_read,
        |callback| unsafe { callback(runtime_id, id, output_event) },
    )
}

/// Create one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_create(
    runtime_id: u64,
    event: HostCalendarEventDraft,
    output_id: *mut HostCalendarEventId,
) -> u32 {
    if output_id.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_create,
        |callback| unsafe { callback(runtime_id, event, output_id) },
    )
}

/// Update one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_update(
    runtime_id: u64,
    id: HostCalendarEventId,
    event: HostCalendarEventDraft,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_update,
        |callback| unsafe { callback(runtime_id, id, event) },
    )
}

/// Delete one Android calendar event through the host.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_calendar_event_delete(
    runtime_id: u64,
    id: HostCalendarEventId,
) -> u32 {
    call_android_calendar_callback(
        runtime_id,
        |callbacks| callbacks.event_delete,
        |callback| unsafe { callback(runtime_id, id) },
    )
}
