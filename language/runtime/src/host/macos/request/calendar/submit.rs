use super::query::{list_calendars, list_events, read_event};
use super::write::{create_event, delete_event, update_event};
use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};

/// Submit one macOS calendar request through EventKit.
pub(crate) fn submit_calendar_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // list calendars
        HostRequest::OsCalendarList => {
            let calendars = list_calendars()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarDescriptors(calendars),
            )))
        }

        // list events
        HostRequest::OsCalendarEventList { query } => {
            let events = list_events(query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvents(events),
            )))
        }

        // read one event
        HostRequest::OsCalendarEventRead { id } => {
            let event = read_event(id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvent(event),
            )))
        }

        // create one event
        HostRequest::OsCalendarEventCreate { event } => {
            let id = create_event(event)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }

        // update one event
        HostRequest::OsCalendarEventUpdate { id, event } => {
            update_event(id, event)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // delete one event
        HostRequest::OsCalendarEventDelete { id } => {
            delete_event(id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
