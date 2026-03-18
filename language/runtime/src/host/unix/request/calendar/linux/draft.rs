use vobject::{Component, Property, write_component};

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarAvailability, CalendarEventDraftValue, CalendarEventValue,
};

use super::event::attendee_property;
use super::provider::create_calendar_objects;
use super::recurrence::recurrence_rule_property;
use super::reminder::reminder_component;
use super::time::datetime_property;
use super::{CALENDAR_EVENT_CREATE_OPERATION, DESTACK_ICAL_PRODID};

/// Build one VCALENDAR payload from one runtime draft.
pub(super) fn calendar_component_from_draft(
    draft: &CalendarEventDraftValue,
    uid: Option<&str>,
    operation: &'static str,
) -> RuntimeResult<Component> {
    let mut calendar = Component::new("VCALENDAR");
    let mut event = Component::new("VEVENT");

    // top-level metadata
    calendar.set(Property::new("VERSION", "2.0"));
    calendar.set(Property::new("PRODID", DESTACK_ICAL_PRODID));

    // identity and summary
    event.set(Property::new(
        "UID",
        uid.ok_or_else(|| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                "calendar event uid is required",
            )
        })?,
    ));
    event.set(Property::new("SUMMARY", &draft.title));

    // scalar metadata
    if let Some(notes) = &draft.notes {
        if !notes.is_empty() {
            event.set(Property::new("DESCRIPTION", notes));
        }
    }

    if let Some(location) = &draft.location {
        if !location.is_empty() {
            event.set(Property::new("LOCATION", location));
        }
    }

    if let Some(url) = &draft.url {
        if !url.is_empty() {
            event.set(Property::new("URL", url));
        }
    }

    // time range
    event.set(datetime_property(
        "DTSTART",
        draft.start_unix_ns,
        draft.all_day,
        draft.time_zone.as_deref(),
        operation,
    )?);
    event.set(datetime_property(
        "DTEND",
        draft.end_unix_ns,
        draft.all_day,
        draft.time_zone.as_deref(),
        operation,
    )?);

    // availability
    match draft.availability {
        CalendarAvailability::Free => event.set(Property::new("TRANSP", "TRANSPARENT")),
        CalendarAvailability::Busy
        | CalendarAvailability::Tentative
        | CalendarAvailability::OutOfOffice
        | CalendarAvailability::Unavailable
        | CalendarAvailability::Unknown => event.set(Property::new("TRANSP", "OPAQUE")),
    }

    // recurrence
    if let Some(recurrence_rule) = &draft.recurrence_rule {
        event.set(recurrence_rule_property(recurrence_rule, operation)?);
    }

    // attendees
    if let Some(attendees) = &draft.attendees {
        for attendee in attendees {
            event.push(attendee_property(attendee));
        }
    }

    // reminders
    if let Some(reminders) = &draft.reminders {
        for reminder in reminders {
            calendar
                .subcomponents
                .push(reminder_component(reminder, operation)?);
        }
    }

    calendar.subcomponents.push(event);

    Ok(calendar)
}

/// Validate one runtime calendar draft.
pub(super) fn validate_event_draft(
    draft: &CalendarEventDraftValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    if draft.title.is_empty() {
        return Err(invalid_argument_value(
            operation,
            "event",
            "calendar event title must not be empty",
        ));
    }

    if draft.calendar_id.is_empty() {
        return Err(invalid_argument_value(
            operation,
            "event",
            "calendar event calendar_id must not be empty",
        ));
    }

    if draft.end_unix_ns <= draft.start_unix_ns {
        return Err(invalid_argument_value(
            operation,
            "event",
            "calendar event end timestamp must be greater than the start timestamp",
        ));
    }

    Ok(())
}

/// Build one stable runtime-generated event uid.
pub(super) fn generated_event_uid(draft: &CalendarEventDraftValue) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);

    format!("destack-{}-{timestamp}", draft.start_unix_ns)
}

/// Create one calendar event and return the created stable identifier.
pub(super) fn create_calendar_event(
    draft: &CalendarEventDraftValue,
    operation: &'static str,
) -> RuntimeResult<String> {
    let uid = generated_event_uid(draft);
    let calendar = calendar_component_from_draft(draft, Some(&uid), operation)?;
    let ids =
        create_calendar_objects(&draft.calendar_id, &[write_component(&calendar)], operation)?;
    let created_uid = ids.into_iter().next().ok_or_else(|| {
        io_operation_error(
            CALENDAR_EVENT_CREATE_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "evolution calendar create returned no created event identifiers",
        )
    })?;

    Ok(created_uid)
}
