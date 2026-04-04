use std::cmp::Ordering;

use vobject::{Component, Property};

use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request::linux::eds::composite_eds_identifier;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarAttendeeValue, CalendarDateTimeValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::platform::os::{CalendarAvailability, CalendarParticipantStatus};

use super::CALENDAR_EVENT_READ_OPERATION;
use super::recurrence::recurrence_rule_from_component;
use super::reminder::reminders_from_component;
use super::time::datetime_value_from_property;

/// Append events from one VCALENDAR payload.
pub(super) fn append_calendar_events(
    events: &mut Vec<CalendarEventValue>,
    source_uid: &str,
    calendar: &Component,
    query: &CalendarEventQueryValue,
) -> RuntimeResult<()> {
    // event materialization
    for subcomponent in &calendar.subcomponents {
        if subcomponent.name != "VEVENT" {
            continue;
        }

        let event = calendar_event_from_component(
            source_uid,
            subcomponent,
            query.include_recurrence_instances,
        )?;
        events.push(event);
    }

    Ok(())
}

/// Return the first VEVENT inside one VCALENDAR payload.
pub(super) fn first_calendar_event<'a>(
    calendar: &'a Component,
    operation: &'static str,
) -> RuntimeResult<&'a Component> {
    calendar
        .subcomponents
        .iter()
        .find(|component| component.name == "VEVENT")
        .ok_or_else(|| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                "evolution calendar returned one calendar payload without one VEVENT",
            )
        })
}

/// Parse one VCALENDAR component payload.
pub(super) fn calendar_component_from_text(
    calendar_text: &str,
    operation: &'static str,
) -> RuntimeResult<Component> {
    let component: Component = calendar_text.parse().map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("evolution calendar returned one invalid iCalendar payload: {error}"),
        )
    })?;

    if component.name != "VCALENDAR" {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "evolution calendar returned one non-VCALENDAR payload",
        ));
    }

    Ok(component)
}

/// Materialize one runtime calendar event from one VEVENT component.
pub(super) fn calendar_event_from_component(
    source_uid: &str,
    event: &Component,
    include_recurrence_instances: bool,
) -> RuntimeResult<CalendarEventValue> {
    let uid = required_property_value(event, "UID", CALENDAR_EVENT_READ_OPERATION)?;
    let id = composite_eds_identifier(source_uid, &uid);
    let start = required_datetime_property(event, "DTSTART", CALENDAR_EVENT_READ_OPERATION)?;
    let end = optional_datetime_property(event, "DTEND", CALENDAR_EVENT_READ_OPERATION)?.unwrap_or(
        CalendarDateTimeValue {
            unix_ns: start.unix_ns,
            is_all_day: start.is_all_day,
            time_zone: start.time_zone.clone(),
        },
    );
    let recurrence_id =
        optional_datetime_property(event, "RECURRENCE-ID", CALENDAR_EVENT_READ_OPERATION)?;
    let recurrence_rule = recurrence_rule_from_component(event)?;
    let reminders = reminders_from_component(event)?;
    let attendees = attendees_from_component(event);
    let organizer_name = organizer_name_from_component(event);
    let organizer_email = organizer_email_from_component(event);
    let status = optional_property_value(event, "STATUS");
    let canceled = status
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("CANCELLED"));
    let recurring = recurrence_rule.is_some() || recurrence_id.is_some();
    let recurrence_master_id = if include_recurrence_instances && recurrence_id.is_some() {
        Some(composite_eds_identifier(source_uid, &uid))
    } else {
        None
    };

    Ok(CalendarEventValue {
        id,
        calendar_id: source_uid.to_string(),
        title: optional_property_value(event, "SUMMARY").unwrap_or_default(),
        notes: optional_property_value(event, "DESCRIPTION"),
        location: optional_property_value(event, "LOCATION"),
        start_unix_ns: start.unix_ns,
        end_unix_ns: end.unix_ns,
        all_day: start.is_all_day,
        canceled,
        time_zone: start.time_zone.clone(),
        availability: availability_from_component(event),
        url: optional_property_value(event, "URL"),
        organizer_name,
        organizer_email,
        recurring,
        recurrence_master_id,
        recurrence_id_unix_ns: recurrence_id.map(|value| value.unix_ns),
        recurrence_rule,
        attendees,
        reminders,
    })
}

/// Apply final query filtering to one event list.
pub(super) fn apply_query_filters(
    events: &mut Vec<CalendarEventValue>,
    query: &CalendarEventQueryValue,
) {
    // time range
    events.retain(|event| {
        event.end_unix_ns > query.start_unix_ns && event.start_unix_ns < query.end_unix_ns
    });

    // canceled events
    if !query.include_canceled {
        events.retain(|event| !event.canceled);
    }

    // recurrence collapsing
    if !query.include_recurrence_instances {
        collapse_recurrence_instances(events);
    }

    // stable ordering
    events.sort_by(compare_events);

    // final limit
    if let Some(limit) = query.limit {
        events.truncate(limit as usize);
    }
}

/// Return one calendar list sexp for the requested range.
pub(super) fn event_list_query(query: &CalendarEventQueryValue) -> String {
    let start = query.start_unix_ns / 1_000_000_000;
    let end = query.end_unix_ns / 1_000_000_000;

    format!("(occur-in-time-range? (make-time \"{start}\") (make-time \"{end}\"))")
}

/// Encode one attendee as one ATTENDEE property.
pub(super) fn attendee_property(attendee: &CalendarAttendeeValue) -> Property {
    let value = attendee
        .email
        .as_ref()
        .map(|email| format!("mailto:{email}"))
        .unwrap_or_default();
    let mut property = Property::new("ATTENDEE", value);

    if let Some(name) = &attendee.name {
        if !name.is_empty() {
            property.params.insert("CN".to_string(), name.clone());
        }
    }

    property.params.insert(
        "ROLE".to_string(),
        if attendee.organizer {
            "CHAIR".to_string()
        } else if attendee.optional {
            "OPT-PARTICIPANT".to_string()
        } else {
            "REQ-PARTICIPANT".to_string()
        },
    );
    property.params.insert(
        "PARTSTAT".to_string(),
        participant_status_token(attendee.response_status).to_string(),
    );

    property
}

/// Return one required scalar property value.
fn required_property_value(
    event: &Component,
    property_name: &str,
    operation: &'static str,
) -> RuntimeResult<String> {
    optional_property_value(event, property_name).ok_or_else(|| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("evolution calendar returned one event without `{property_name}`"),
        )
    })
}

/// Return one optional scalar property value.
fn optional_property_value(event: &Component, property_name: &str) -> Option<String> {
    event.get_only(property_name).map(Property::value_as_string)
}

/// Return one required datetime property.
fn required_datetime_property(
    event: &Component,
    property_name: &str,
    operation: &'static str,
) -> RuntimeResult<CalendarDateTimeValue> {
    let property = event.get_only(property_name).ok_or_else(|| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("evolution calendar returned one event without `{property_name}`"),
        )
    })?;

    datetime_value_from_property(property, operation)
}

/// Return one optional datetime property.
fn optional_datetime_property(
    event: &Component,
    property_name: &str,
    operation: &'static str,
) -> RuntimeResult<Option<CalendarDateTimeValue>> {
    let Some(property) = event.get_only(property_name) else {
        return Ok(None);
    };

    datetime_value_from_property(property, operation).map(Some)
}

/// Decode one attendee list from one VEVENT component.
fn attendees_from_component(event: &Component) -> Option<Vec<CalendarAttendeeValue>> {
    let mut attendees = Vec::new();

    // attendee materialization
    for property in event.get_all("ATTENDEE") {
        let value = property.value_as_string();
        let email = value
            .strip_prefix("mailto:")
            .or_else(|| value.strip_prefix("MAILTO:"))
            .map(ToOwned::to_owned);

        attendees.push(CalendarAttendeeValue {
            id: None,
            name: property.params.get("CN").cloned(),
            email,
            optional: property
                .params
                .get("ROLE")
                .is_some_and(|value| value.eq_ignore_ascii_case("OPT-PARTICIPANT")),
            organizer: property
                .params
                .get("ROLE")
                .is_some_and(|value| value.eq_ignore_ascii_case("CHAIR")),
            response_status: participant_status_from_raw(property.params.get("PARTSTAT")),
        });
    }

    if attendees.is_empty() {
        return None;
    }

    Some(attendees)
}

/// Return one organizer display name from one VEVENT.
fn organizer_name_from_component(event: &Component) -> Option<String> {
    event
        .get_only("ORGANIZER")
        .and_then(|property| property.params.get("CN").cloned())
}

/// Return one organizer email from one VEVENT.
fn organizer_email_from_component(event: &Component) -> Option<String> {
    let property = event.get_only("ORGANIZER")?;
    let value = property.value_as_string();

    value
        .strip_prefix("mailto:")
        .or_else(|| value.strip_prefix("MAILTO:"))
        .map(ToOwned::to_owned)
}

/// Return one availability value from one VEVENT.
fn availability_from_component(event: &Component) -> CalendarAvailability {
    let status = optional_property_value(event, "STATUS");
    let transparency = optional_property_value(event, "TRANSP");

    if transparency
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("TRANSPARENT"))
    {
        return CalendarAvailability::Free;
    }

    match status.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("TENTATIVE") => CalendarAvailability::Tentative,
        Some(value) if value.eq_ignore_ascii_case("CANCELLED") => CalendarAvailability::Unavailable,
        _ => CalendarAvailability::Busy,
    }
}

/// Collapse expanded recurrence instances into one row per series.
fn collapse_recurrence_instances(events: &mut Vec<CalendarEventValue>) {
    let mut seen_series = std::collections::BTreeSet::new();

    events.retain(|event| {
        if !event.recurring {
            return true;
        }

        let series_key = event
            .recurrence_master_id
            .clone()
            .unwrap_or_else(|| event.id.clone());

        seen_series.insert(series_key)
    });
}

/// Return one stable event ordering.
fn compare_events(left: &CalendarEventValue, right: &CalendarEventValue) -> Ordering {
    left.start_unix_ns
        .cmp(&right.start_unix_ns)
        .then_with(|| left.end_unix_ns.cmp(&right.end_unix_ns))
        .then_with(|| left.id.cmp(&right.id))
}

/// Return one participant status from one raw PARTSTAT token.
fn participant_status_from_raw(value: Option<&String>) -> CalendarParticipantStatus {
    match value.map(String::as_str) {
        Some(value) if value.eq_ignore_ascii_case("ACCEPTED") => {
            CalendarParticipantStatus::Accepted
        }
        Some(value) if value.eq_ignore_ascii_case("TENTATIVE") => {
            CalendarParticipantStatus::Tentative
        }
        Some(value) if value.eq_ignore_ascii_case("DECLINED") => {
            CalendarParticipantStatus::Declined
        }
        Some(value) if value.eq_ignore_ascii_case("DELEGATED") => {
            CalendarParticipantStatus::Delegated
        }
        Some(value) if value.eq_ignore_ascii_case("COMPLETED") => {
            CalendarParticipantStatus::Completed
        }
        Some(value) if value.eq_ignore_ascii_case("IN-PROCESS") => {
            CalendarParticipantStatus::InProcess
        }
        Some(value) if value.eq_ignore_ascii_case("NEEDS-ACTION") => {
            CalendarParticipantStatus::Pending
        }
        _ => CalendarParticipantStatus::Unknown,
    }
}

/// Return one raw PARTSTAT token for one runtime status.
fn participant_status_token(status: CalendarParticipantStatus) -> &'static str {
    match status {
        CalendarParticipantStatus::Unknown => "NEEDS-ACTION",
        CalendarParticipantStatus::Pending => "NEEDS-ACTION",
        CalendarParticipantStatus::Accepted => "ACCEPTED",
        CalendarParticipantStatus::Tentative => "TENTATIVE",
        CalendarParticipantStatus::Declined => "DECLINED",
        CalendarParticipantStatus::Delegated => "DELEGATED",
        CalendarParticipantStatus::Completed => "COMPLETED",
        CalendarParticipantStatus::InProcess => "IN-PROCESS",
    }
}
