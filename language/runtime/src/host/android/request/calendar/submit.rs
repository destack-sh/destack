use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::calendar::{
    HostCalendarDescriptorArray, HostCalendarEvent, HostCalendarEventArray, HostCalendarEventId,
    decode_calendar_descriptors, decode_calendar_event, decode_calendar_events,
    encode_calendar_event_draft, encode_calendar_event_query,
};
use crate::host::android::abi::calendar::{
    destack_host_android_calendar_event_create, destack_host_android_calendar_event_delete,
    destack_host_android_calendar_event_list, destack_host_android_calendar_event_read,
    destack_host_android_calendar_event_update, destack_host_android_calendar_list,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::runtime::{BindingCallContext, NativeStringRef};

/// Return one Android calendar request outcome when supported.
pub(crate) fn submit_calendar_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsCalendarList => {
            let mut calendars = MaybeUninit::<HostCalendarDescriptorArray>::uninit();
            let status =
                unsafe { destack_host_android_calendar_list(runtime_id, calendars.as_mut_ptr()) };
            decode_callback_host_status(status, request.operation_name())?;

            let calendars = unsafe { calendars.assume_init() };
            let calendars = unsafe { decode_calendar_descriptors(calendars) }?;

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
            let call_status = unsafe {
                destack_host_android_calendar_event_delete(runtime_id, NativeStringRef::from(id))
            };
            decode_callback_host_status(call_status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Submit one Android calendar event-list request.
fn submit_calendar_event_list(
    runtime_id: u64,
    operation: &'static str,
    query: &CalendarEventQueryValue,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let query = encode_calendar_event_query(&binding, query);
    let mut output_events = MaybeUninit::<HostCalendarEventArray>::uninit();
    let status = unsafe {
        destack_host_android_calendar_event_list(runtime_id, query, output_events.as_mut_ptr())
    };
    decode_callback_host_status(status, operation)?;

    let output_events = unsafe { output_events.assume_init() };
    unsafe { decode_calendar_events(output_events) }
}

/// Submit one Android calendar event-read request.
fn submit_calendar_event_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<CalendarEventValue> {
    let mut output_event = MaybeUninit::<HostCalendarEvent>::uninit();
    let status = unsafe {
        destack_host_android_calendar_event_read(
            runtime_id,
            NativeStringRef::from(id),
            output_event.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let output_event = unsafe { output_event.assume_init() };
    unsafe { decode_calendar_event(output_event) }
}

/// Submit one Android calendar event-create request.
fn submit_calendar_event_create(
    runtime_id: u64,
    operation: &'static str,
    event: &CalendarEventDraftValue,
) -> RuntimeResult<String> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let event = encode_calendar_event_draft(&binding, event);
    let mut output_id = MaybeUninit::<HostCalendarEventId>::uninit();
    let status = unsafe {
        destack_host_android_calendar_event_create(runtime_id, event, output_id.as_mut_ptr())
    };
    decode_callback_host_status(status, operation)?;

    let output_id = unsafe { output_id.assume_init() };
    Ok(unsafe { output_id.as_str()? }.to_string())
}

/// Submit one Android calendar event-update request.
fn submit_calendar_event_update(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
    event: &CalendarEventDraftValue,
) -> RuntimeResult<()> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let event = encode_calendar_event_draft(&binding, event);
    let call_status = unsafe {
        destack_host_android_calendar_event_update(runtime_id, NativeStringRef::from(id), event)
    };
    decode_callback_host_status(call_status, operation)
}
