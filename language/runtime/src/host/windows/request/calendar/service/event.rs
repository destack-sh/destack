use windows::ApplicationModel::Appointments::Appointment;

use super::core::{
    duration_ns_from_timespan, optional_hstring, unix_ns_from_datetime, windows_calendar_error,
};
use super::recurrence::{
    availability_from_busy_status, participant_status_from_response, recurrence_rule_from_native,
    reminders_from_native,
};
use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::os::CalendarParticipantStatus;
use crate::platform::os::abi_generated::{
    CalendarAttendeeValue, CalendarEventQueryValue, CalendarEventValue,
};

/// Decode one native appointment into one runtime event.
pub(super) fn calendar_event_from_native(
    appointment: &Appointment,
    operation: &'static str,
) -> RuntimeResult<CalendarEventValue> {
    let start_unix_ns =
        unix_ns_from_datetime(appointment.StartTime().map_err(|error| {
            windows_calendar_error(operation, "Appointment::StartTime", &error)
        })?);
    let duration_ns = duration_ns_from_timespan(
        appointment
            .Duration()
            .map_err(|error| windows_calendar_error(operation, "Appointment::Duration", &error))?,
    );
    let recurrence_rule = recurrence_rule_from_native(appointment, operation)?;
    let reminders = reminders_from_native(appointment, operation)?;
    let attendees = attendees_from_native(appointment, operation)?;
    let organizer = appointment
        .Organizer()
        .map_err(|error| windows_calendar_error(operation, "Appointment::Organizer", &error))?;

    Ok(CalendarEventValue {
        id: appointment
            .LocalId()
            .map_err(|error| windows_calendar_error(operation, "Appointment::LocalId", &error))?
            .to_string(),
        calendar_id: appointment
            .CalendarId()
            .map_err(|error| windows_calendar_error(operation, "Appointment::CalendarId", &error))?
            .to_string(),
        title: appointment
            .Subject()
            .map_err(|error| windows_calendar_error(operation, "Appointment::Subject", &error))?
            .to_string(),
        notes: optional_hstring(
            appointment.Details().map_err(|error| {
                windows_calendar_error(operation, "Appointment::Details", &error)
            })?,
        ),
        location: optional_hstring(
            appointment.Location().map_err(|error| {
                windows_calendar_error(operation, "Appointment::Location", &error)
            })?,
        ),
        start_unix_ns,
        end_unix_ns: start_unix_ns.saturating_add(duration_ns),
        all_day: appointment
            .AllDay()
            .map_err(|error| windows_calendar_error(operation, "Appointment::AllDay", &error))?,
        canceled: false,
        time_zone: None,
        availability: availability_from_busy_status(appointment.BusyStatus().map_err(|error| {
            windows_calendar_error(operation, "Appointment::BusyStatus", &error)
        })?),
        url: match appointment.Uri() {
            Ok(value) => Some(
                value
                    .RawUri()
                    .map_err(|error| windows_calendar_error(operation, "Uri::RawUri", &error))?
                    .to_string(),
            ),
            Err(_) => None,
        },
        organizer_name: optional_hstring(organizer.DisplayName().map_err(|error| {
            windows_calendar_error(operation, "AppointmentOrganizer::DisplayName", &error)
        })?),
        organizer_email: optional_hstring(organizer.Address().map_err(|error| {
            windows_calendar_error(operation, "AppointmentOrganizer::Address", &error)
        })?),
        recurring: recurrence_rule.is_some(),
        recurrence_master_id: None,
        recurrence_id_unix_ns: appointment
            .OriginalStartTime()
            .ok()
            .and_then(|value| value.Value().ok())
            .map(unix_ns_from_datetime),
        recurrence_rule,
        attendees,
        reminders,
    })
}

/// Return the runtime attendee list from the native appointment.
fn attendees_from_native(
    appointment: &Appointment,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<CalendarAttendeeValue>>> {
    let invitees = appointment
        .Invitees()
        .map_err(|error| windows_calendar_error(operation, "Appointment::Invitees", &error))?;
    let count = invitees
        .Size()
        .map_err(|error| windows_calendar_error(operation, "IVector::Size", &error))?;

    if count == 0 {
        return Ok(None);
    }

    let mut attendees = Vec::with_capacity(count as usize);

    // decode one invitee per attendee row
    for index in 0..count {
        let invitee = invitees
            .GetAt(index)
            .map_err(|error| windows_calendar_error(operation, "IVector::GetAt", &error))?;

        attendees.push(CalendarAttendeeValue {
            id: None,
            name: optional_hstring(invitee.DisplayName().map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::DisplayName", &error)
            })?),
            email: optional_hstring(invitee.Address().map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::Address", &error)
            })?),
            optional: invitee.Role().map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::Role", &error)
            })? == windows::ApplicationModel::Appointments::AppointmentParticipantRole::OptionalAttendee,
            organizer: false,
            response_status: participant_status_from_response(invitee.Response().map_err(
                |error| windows_calendar_error(operation, "AppointmentInvitee::Response", &error),
            )?),
        });
    }

    Ok(Some(attendees))
}

/// Return whether one event overlaps the requested range.
pub(super) fn event_overlaps_query(
    event: &CalendarEventValue,
    start_unix_ns: u64,
    end_unix_ns: u64,
) -> bool {
    event.start_unix_ns < end_unix_ns && event.end_unix_ns > start_unix_ns
}

/// Return whether the current user declined one event.
pub(super) fn is_declined_for_current_user(event: &CalendarEventValue) -> bool {
    let Some(attendees) = &event.attendees else {
        return false;
    };

    attendees.iter().any(|attendee| {
        attendee.organizer || attendee.response_status == CalendarParticipantStatus::Declined
    })
}

/// Return one query range length or fail on invalid bounds.
pub(super) fn range_length_for_query(
    query: &CalendarEventQueryValue,
) -> RuntimeResult<windows::Foundation::TimeSpan> {
    if query.end_unix_ns <= query.start_unix_ns {
        return Err(invalid_argument_value(
            "query.end_unix_ns",
            "calendar query end timestamp must be greater than the start timestamp",
        ));
    }

    Ok(super::core::timespan_from_ns(
        (query.end_unix_ns - query.start_unix_ns) as i64,
    ))
}
