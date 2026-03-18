use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};

/// Build one calendar list operation.
pub(crate) fn list() -> HostOperation<Vec<CalendarDescriptorValue>> {
    HostOperation::new(HostRequest::OsCalendarList, decode::calendar_descriptors)
}

/// Build one calendar event list operation.
pub(crate) fn event_list(query: CalendarEventQueryValue) -> HostOperation<Vec<CalendarEventValue>> {
    HostOperation::new(
        HostRequest::OsCalendarEventList { query },
        decode::calendar_events,
    )
}

/// Build one calendar event read operation.
pub(crate) fn event_read(id: String) -> HostOperation<CalendarEventValue> {
    HostOperation::new(
        HostRequest::OsCalendarEventRead { id },
        decode::calendar_event,
    )
}

/// Build one calendar event create operation.
pub(crate) fn event_create(event: CalendarEventDraftValue) -> HostOperation<String> {
    HostOperation::new(HostRequest::OsCalendarEventCreate { event }, decode::string)
}

/// Build one calendar event update operation.
pub(crate) fn event_update(id: String, event: CalendarEventDraftValue) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsCalendarEventUpdate { id, event },
        decode::none,
    )
}

/// Build one calendar event delete operation.
pub(crate) fn event_delete(id: String) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsCalendarEventDelete { id }, decode::none)
}
