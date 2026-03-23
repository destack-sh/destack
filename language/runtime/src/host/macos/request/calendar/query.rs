use objc2_foundation::{NSArray, NSString};

use super::core::{
    CALENDAR_EVENT_LIST_OPERATION, CALENDAR_EVENT_READ_OPERATION, collapse_recurrence_instances,
    new_event_store, ns_date_from_unix_ns,
};
use super::native::{
    calendar_descriptor_from_native, calendar_event_value_from_native, default_calendar_id,
    is_declined_for_current_user_native,
};
use crate::diagnostic::RuntimeResult;
use crate::host::apple::core::execution::call_process_main_context_if_needed;
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventQueryValue, CalendarEventValue,
};

/// List calendars through one fresh EventKit store.
pub(super) fn list_calendars() -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    call_process_main_context_if_needed(|| {
        let store = new_event_store();
        let default_calendar_id = default_calendar_id(store.as_ref());
        let calendars =
            unsafe { store.calendarsForEntityType(objc2_event_kit::EKEntityType::Event) };
        let mut descriptors = Vec::with_capacity(calendars.len());

        // materialize one runtime descriptor per native calendar
        for calendar in calendars.iter() {
            let descriptor =
                calendar_descriptor_from_native(calendar.as_ref(), default_calendar_id.as_deref())?;

            descriptors.push(descriptor);
        }

        Ok(descriptors)
    })
}

/// List calendar events through one EventKit query.
pub(super) fn list_events(
    query: &CalendarEventQueryValue,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    let query = query.clone();

    call_process_main_context_if_needed(move || {
        let store = new_event_store();
        let predicate = predicate_for_query(store.as_ref(), &query)?;
        let native_events = unsafe { store.eventsMatchingPredicate(predicate.as_ref()) };
        let mut events = Vec::with_capacity(native_events.len());

        // materialize one runtime event per native match
        for event in native_events.iter() {
            if !query.include_declined && is_declined_for_current_user_native(event.as_ref()) {
                continue;
            }

            let event = calendar_event_value_from_native(event.as_ref())?;
            events.push(event);
        }

        // query filters
        apply_query_filters(&mut events, &query);

        Ok(events)
    })
}

/// Read one EventKit event by its stable event identifier.
pub(super) fn read_event(id: &str) -> RuntimeResult<CalendarEventValue> {
    let id = id.to_string();

    call_process_main_context_if_needed(move || {
        let store = new_event_store();
        let event = resolve_event(store.as_ref(), &id, CALENDAR_EVENT_READ_OPERATION)?;

        calendar_event_value_from_native(event.as_ref())
    })
}

/// Apply high-level query filters after EventKit expansion.
fn apply_query_filters(events: &mut Vec<CalendarEventValue>, query: &CalendarEventQueryValue) {
    // canceled rows
    if !query.include_canceled {
        events.retain(|event| !event.canceled);
    }

    // EventKit expands recurrence instances for range queries, so collapse them explicitly
    if !query.include_recurrence_instances {
        collapse_recurrence_instances(events);
    }

    // stable ordering before limit
    events.sort_by(|left, right| {
        left.start_unix_ns
            .cmp(&right.start_unix_ns)
            .then_with(|| left.end_unix_ns.cmp(&right.end_unix_ns))
            .then_with(|| left.id.cmp(&right.id))
    });

    // final limit
    if let Some(limit) = query.limit {
        events.truncate(limit as usize);
    }
}

/// Build one native predicate for one runtime event query.
fn predicate_for_query(
    store: &objc2_event_kit::EKEventStore,
    query: &CalendarEventQueryValue,
) -> RuntimeResult<objc2::rc::Retained<objc2_foundation::NSPredicate>> {
    let start_date = ns_date_from_unix_ns(query.start_unix_ns);
    let end_date = ns_date_from_unix_ns(query.end_unix_ns);
    let calendars =
        resolve_query_calendars(store, &query.calendar_ids, CALENDAR_EVENT_LIST_OPERATION)?;

    // EventKit range queries are synchronous and expand recurring instances
    let predicate = unsafe {
        store.predicateForEventsWithStartDate_endDate_calendars(
            start_date.as_ref(),
            end_date.as_ref(),
            calendars.as_deref(),
        )
    };

    Ok(predicate)
}

/// Resolve one optional calendar filter list.
fn resolve_query_calendars(
    store: &objc2_event_kit::EKEventStore,
    calendar_ids: &[String],
    operation: &'static str,
) -> RuntimeResult<Option<objc2::rc::Retained<NSArray<objc2_event_kit::EKCalendar>>>> {
    if calendar_ids.is_empty() {
        return Ok(None);
    }

    let mut calendars = Vec::with_capacity(calendar_ids.len());

    // resolve one EventKit calendar per runtime identifier
    for calendar_id in calendar_ids {
        let calendar = resolve_calendar(store, calendar_id, operation)?;
        calendars.push(calendar);
    }

    Ok(Some(NSArray::from_retained_slice(&calendars)))
}

/// Resolve one calendar by identifier.
pub(super) fn resolve_calendar(
    store: &objc2_event_kit::EKEventStore,
    calendar_id: &str,
    operation: &'static str,
) -> RuntimeResult<objc2::rc::Retained<objc2_event_kit::EKCalendar>> {
    let identifier = NSString::from_str(calendar_id);
    let calendar = unsafe { store.calendarWithIdentifier(&identifier) };

    calendar.ok_or_else(|| {
        super::core::calendar_error(
            operation,
            format!("unknown macOS calendar identifier `{calendar_id}`"),
        )
    })
}

/// Resolve one event by identifier.
pub(super) fn resolve_event(
    store: &objc2_event_kit::EKEventStore,
    event_id: &str,
    operation: &'static str,
) -> RuntimeResult<objc2::rc::Retained<objc2_event_kit::EKEvent>> {
    let identifier = NSString::from_str(event_id);
    let event = unsafe { store.eventWithIdentifier(&identifier) };

    event.ok_or_else(|| {
        super::core::calendar_error(
            operation,
            format!("unknown macOS calendar event identifier `{event_id}`"),
        )
    })
}
