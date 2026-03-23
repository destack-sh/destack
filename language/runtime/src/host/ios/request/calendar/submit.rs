use crate::diagnostic::RuntimeResult;
use crate::host::core::callback::{
    decode_callback_host_status, encode_callback_host_json, read_buffered_callback_host_json,
    read_buffered_callback_host_string,
};
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::host::ios::abi::calendar::{
    destack_host_ios_calendar_event_create, destack_host_ios_calendar_event_delete,
    destack_host_ios_calendar_event_list, destack_host_ios_calendar_event_read,
    destack_host_ios_calendar_event_update, destack_host_ios_calendar_list,
};
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::runtime::NativeSlice;

/// Return one iOS calendar request outcome when supported.
pub(crate) fn submit_calendar_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsCalendarList => {
            let calendars = read_buffered_callback_host_json::<Vec<CalendarDescriptorValue>>(
                request.operation_name(),
                "calendar descriptors",
                |output, output_written| unsafe {
                    destack_host_ios_calendar_list(runtime_id, output, output_written)
                },
            )?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarDescriptors(calendars),
            )))
        }
        HostRequest::OsCalendarEventList { query } => {
            let events = submit_calendar_event_list(runtime_id, request.operation_name(), query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvents(events),
            )))
        }
        HostRequest::OsCalendarEventRead { id } => {
            let event = submit_calendar_event_read(runtime_id, request.operation_name(), id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvent(event),
            )))
        }
        HostRequest::OsCalendarEventCreate { event } => {
            let id = submit_calendar_event_create(runtime_id, request.operation_name(), event)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsCalendarEventUpdate { id, event } => {
            submit_calendar_event_update(runtime_id, request.operation_name(), id, event)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsCalendarEventDelete { id } => {
            let id = id.as_bytes();
            let call_status = unsafe {
                destack_host_ios_calendar_event_delete(
                    runtime_id,
                    NativeSlice {
                        data: id.as_ptr() as *mut u8,
                        len: id.len() as u32,
                    },
                )
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Submit one iOS calendar event-list request.
fn submit_calendar_event_list(
    runtime_id: u64,
    operation: &'static str,
    query: &CalendarEventQueryValue,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    let payload = encode_callback_host_json(query, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };

    read_buffered_callback_host_json(
        operation,
        "calendar events",
        |output, output_written| unsafe {
            destack_host_ios_calendar_event_list(runtime_id, payload, output, output_written)
        },
    )
}

/// Submit one iOS calendar event-read request.
fn submit_calendar_event_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<CalendarEventValue> {
    let id = id.as_bytes();
    let id = NativeSlice {
        data: id.as_ptr() as *mut u8,
        len: id.len() as u32,
    };

    read_buffered_callback_host_json(
        operation,
        "calendar event",
        |output, output_written| unsafe {
            destack_host_ios_calendar_event_read(runtime_id, id, output, output_written)
        },
    )
}

/// Submit one iOS calendar event-create request.
fn submit_calendar_event_create(
    runtime_id: u64,
    operation: &'static str,
    event: &CalendarEventDraftValue,
) -> RuntimeResult<String> {
    let payload = encode_callback_host_json(event, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };

    read_buffered_callback_host_string(operation, |output_id, output_written| unsafe {
        destack_host_ios_calendar_event_create(runtime_id, payload, output_id, output_written)
    })
}

/// Submit one iOS calendar event-update request.
fn submit_calendar_event_update(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
    event: &CalendarEventDraftValue,
) -> RuntimeResult<()> {
    let id = id.as_bytes();
    let id = NativeSlice {
        data: id.as_ptr() as *mut u8,
        len: id.len() as u32,
    };
    let payload = encode_callback_host_json(event, operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload.len() as u32,
    };
    let call_status = unsafe { destack_host_ios_calendar_event_update(runtime_id, id, payload) };
    decode_callback_host_status(call_status, operation)
}
