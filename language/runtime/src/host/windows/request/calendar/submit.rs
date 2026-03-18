use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};

use super::service::windows_calendar_service;

/// Submit one Windows calendar request through the shared WinRT calendar service.
pub(crate) fn submit_calendar_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let service = windows_calendar_service(request.operation_name())?;

    match request {
        // list calendars
        HostRequest::OsCalendarList => {
            let calendars = service.list_calendars(request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarDescriptors(calendars),
            )))
        }

        // list one query range of events
        HostRequest::OsCalendarEventList { query } => {
            let events = service.list_events(query, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvents(events),
            )))
        }

        // read one event
        HostRequest::OsCalendarEventRead { id } => {
            let event = service.read_event(id, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvent(event),
            )))
        }

        // create one event
        HostRequest::OsCalendarEventCreate { event } => {
            let id = service.create_event(event, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }

        // update one event
        HostRequest::OsCalendarEventUpdate { id, event } => {
            service.update_event(id, event, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // delete one event
        HostRequest::OsCalendarEventDelete { id } => {
            service.delete_event(id, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
