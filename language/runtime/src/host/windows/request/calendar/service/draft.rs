use windows::ApplicationModel::Appointments::{
    Appointment, AppointmentInvitee, AppointmentParticipantRole,
};
use windows::core::HSTRING;

use super::core::{duration_timespan_from_bounds, windows_calendar_error};
use super::recurrence::{
    apply_recurrence_rule, apply_reminders, busy_status_from_availability,
    participant_response_from_value,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{CalendarAttendeeValue, CalendarEventDraftValue};

/// Apply one runtime draft to one native appointment.
pub(super) fn apply_event_draft(
    appointment: &Appointment,
    draft: &CalendarEventDraftValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    // scalar fields
    appointment
        .SetSubject(&HSTRING::from(&draft.title))
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetSubject", &error))?;
    appointment
        .SetDetails(&HSTRING::from(draft.notes.as_deref().unwrap_or_default()))
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetDetails", &error))?;
    appointment
        .SetLocation(&HSTRING::from(
            draft.location.as_deref().unwrap_or_default(),
        ))
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetLocation", &error))?;
    appointment
        .SetStartTime(super::core::datetime_from_unix_ns(draft.start_unix_ns))
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetStartTime", &error))?;
    appointment
        .SetDuration(duration_timespan_from_bounds(
            draft.start_unix_ns,
            draft.end_unix_ns,
        )?)
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetDuration", &error))?;
    appointment
        .SetAllDay(draft.all_day)
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetAllDay", &error))?;
    appointment
        .SetBusyStatus(busy_status_from_availability(draft.availability))
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetBusyStatus", &error))?;

    // optional metadata
    if let Some(url) = &draft.url {
        let url = windows::Foundation::Uri::CreateUri(&HSTRING::from(url))
            .map_err(|error| windows_calendar_error(operation, "Uri::CreateUri", &error))?;

        appointment
            .SetUri(&url)
            .map_err(|error| windows_calendar_error(operation, "Appointment::SetUri", &error))?;
    }

    // recurrence, attendees, reminders
    apply_recurrence_rule(appointment, draft.recurrence_rule.as_ref(), operation)?;
    apply_attendees(appointment, draft.attendees.as_deref(), operation)?;
    apply_reminders(appointment, draft.reminders.as_deref(), operation)?;

    Ok(())
}

/// Apply one optional attendee list to one native appointment.
fn apply_attendees(
    appointment: &Appointment,
    attendees: Option<&[CalendarAttendeeValue]>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let invitees = appointment
        .Invitees()
        .map_err(|error| windows_calendar_error(operation, "Appointment::Invitees", &error))?;
    invitees
        .Clear()
        .map_err(|error| windows_calendar_error(operation, "IVector::Clear", &error))?;

    let Some(attendees) = attendees else {
        return Ok(());
    };

    // append one native invitee per runtime attendee
    for attendee in attendees {
        let invitee = AppointmentInvitee::new().map_err(|error| {
            windows_calendar_error(operation, "AppointmentInvitee::new", &error)
        })?;

        invitee
            .SetDisplayName(&HSTRING::from(attendee.name.as_deref().unwrap_or_default()))
            .map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::SetDisplayName", &error)
            })?;
        invitee
            .SetAddress(&HSTRING::from(
                attendee.email.as_deref().unwrap_or_default(),
            ))
            .map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::SetAddress", &error)
            })?;
        invitee
            .SetRole(if attendee.optional {
                AppointmentParticipantRole::OptionalAttendee
            } else {
                AppointmentParticipantRole::RequiredAttendee
            })
            .map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::SetRole", &error)
            })?;
        invitee
            .SetResponse(participant_response_from_value(attendee.response_status))
            .map_err(|error| {
                windows_calendar_error(operation, "AppointmentInvitee::SetResponse", &error)
            })?;
        invitees
            .Append(&invitee)
            .map_err(|error| windows_calendar_error(operation, "IVector::Append", &error))?;
    }

    Ok(())
}
