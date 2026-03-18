use objc2_event_kit::{
    EKCalendar, EKEvent, EKEventStatus, EKParticipant, EKParticipantRole, EKParticipantStatus,
};

use super::core::{
    CALENDAR_EVENT_READ_OPERATION, calendar_error, float_channel, unix_ns_from_date,
};
use super::recurrence::{
    calendar_availability_from_native, participant_status_from_native, recurrence_rule_from_native,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    CalendarAccess, CalendarAttendeeValue, CalendarDescriptorValue, CalendarEventValue,
    CalendarReminderValue,
};

/// Return the default macOS event calendar identifier when one exists.
pub(super) fn default_calendar_id(store: &objc2_event_kit::EKEventStore) -> Option<String> {
    let default_calendar = unsafe { store.defaultCalendarForNewEvents() }?;

    Some(unsafe { default_calendar.calendarIdentifier() }.to_string())
}

/// Build one runtime descriptor from one native calendar.
pub(super) fn calendar_descriptor_from_native(
    calendar: &EKCalendar,
    default_calendar_id: Option<&str>,
) -> RuntimeResult<CalendarDescriptorValue> {
    let id = unsafe { calendar.calendarIdentifier() }.to_string();
    let title = unsafe { calendar.title() }.to_string();
    let source = unsafe { calendar.source() }
        .map(|source| unsafe { source.title() }.to_string())
        .unwrap_or_else(|| "Local".to_string());
    let access = if unsafe { calendar.allowsContentModifications() } {
        CalendarAccess::Write
    } else {
        CalendarAccess::Read
    };
    let primary = default_calendar_id.is_some_and(|default_id| default_id == id);
    let color_argb = color_argb_from_calendar(calendar);

    Ok(CalendarDescriptorValue {
        id,
        title,
        source,
        owner: None,
        color_argb,
        primary,
        access,
    })
}

/// Encode one native calendar color into ARGB bytes.
fn color_argb_from_calendar(calendar: &EKCalendar) -> u32 {
    let color = unsafe { calendar.color() };
    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;
    let mut alpha = 1.0;

    // decode one calibrated AppKit color
    unsafe {
        color.getRed_green_blue_alpha(&mut red, &mut green, &mut blue, &mut alpha);
    }

    let alpha = float_channel(alpha);
    let red = float_channel(red);
    let green = float_channel(green);
    let blue = float_channel(blue);

    ((alpha as u32) << 24) | ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

/// Build one runtime event from one native EventKit event.
pub(super) fn calendar_event_value_from_native(
    event: &EKEvent,
) -> RuntimeResult<CalendarEventValue> {
    let id = unsafe { event.eventIdentifier() }
        .ok_or_else(|| {
            calendar_error(
                CALENDAR_EVENT_READ_OPERATION,
                "native EventKit event is missing one stable identifier",
            )
        })?
        .to_string();
    let calendar = unsafe { event.calendar() }.ok_or_else(|| {
        calendar_error(
            CALENDAR_EVENT_READ_OPERATION,
            "native EventKit event is missing one calendar",
        )
    })?;
    let calendar_id = unsafe { calendar.calendarIdentifier() }.to_string();
    let recurrence_rule = recurrence_rule_from_native(event)?;
    let recurrence_master_id =
        unsafe { event.calendarItemExternalIdentifier() }.map(|value| value.to_string());
    let recurrence_id_unix_ns =
        unsafe { event.occurrenceDate() }.map(|value| unix_ns_from_date(value.as_ref()));
    let attendees = attendees_from_native(event)?;
    let reminders = reminders_from_native(event)?;
    let organizer = unsafe { event.organizer() };
    let organizer_name = organizer
        .as_ref()
        .and_then(|participant| unsafe { participant.name() })
        .map(|value| value.to_string());
    let organizer_email = organizer
        .as_ref()
        .and_then(|participant| participant_email(participant.as_ref()));
    let availability = calendar_availability_from_native(unsafe { event.availability() });
    let canceled = unsafe { event.status() } == EKEventStatus::Canceled;
    let start_date = unsafe { event.startDate() };
    let end_date = unsafe { event.endDate() };

    Ok(CalendarEventValue {
        id,
        calendar_id,
        title: unsafe { event.title() }.to_string(),
        notes: unsafe { event.notes() }.map(|value| value.to_string()),
        location: unsafe { event.location() }.map(|value| value.to_string()),
        start_unix_ns: unix_ns_from_date(start_date.as_ref()),
        end_unix_ns: unix_ns_from_date(end_date.as_ref()),
        all_day: unsafe { event.isAllDay() },
        canceled,
        time_zone: unsafe { event.timeZone() }.map(|value| value.name().to_string()),
        availability,
        url: unsafe { event.URL() }
            .and_then(|value| value.absoluteString())
            .map(|value| value.to_string()),
        organizer_name,
        organizer_email,
        recurring: recurrence_rule.is_some() || recurrence_id_unix_ns.is_some(),
        recurrence_master_id,
        recurrence_id_unix_ns,
        recurrence_rule,
        attendees,
        reminders,
    })
}

/// Build one runtime attendee list from one native event.
fn attendees_from_native(event: &EKEvent) -> RuntimeResult<Option<Vec<CalendarAttendeeValue>>> {
    let Some(attendees) = (unsafe { event.attendees() }) else {
        return Ok(None);
    };
    let mut values = Vec::with_capacity(attendees.len());

    // materialize one attendee row per native participant
    for attendee in attendees.iter() {
        values.push(attendee_from_native(attendee.as_ref()));
    }

    Ok(Some(values))
}

/// Build one runtime attendee from one native participant.
fn attendee_from_native(attendee: &EKParticipant) -> CalendarAttendeeValue {
    let role = unsafe { attendee.participantRole() };
    let status = unsafe { attendee.participantStatus() };

    CalendarAttendeeValue {
        id: unsafe { attendee.URL() }
            .absoluteString()
            .map(|value| value.to_string()),
        name: unsafe { attendee.name() }.map(|value| value.to_string()),
        email: participant_email(attendee),
        optional: role == EKParticipantRole::Optional,
        organizer: role == EKParticipantRole::Chair,
        response_status: participant_status_from_native(status),
    }
}

/// Extract one participant email when the native URL is a mailto address.
fn participant_email(attendee: &EKParticipant) -> Option<String> {
    let value = unsafe { attendee.URL() }.absoluteString()?.to_string();
    let email = value.strip_prefix("mailto:")?;

    Some(email.to_string())
}

/// Build one runtime reminder list from one native event.
fn reminders_from_native(event: &EKEvent) -> RuntimeResult<Option<Vec<CalendarReminderValue>>> {
    let Some(alarms) = (unsafe { event.alarms() }) else {
        return Ok(None);
    };
    let mut values = Vec::with_capacity(alarms.len());

    // materialize one reminder row per native alarm
    for alarm in alarms.iter() {
        let reminder = super::recurrence::reminder_from_native(alarm.as_ref())?;
        values.push(reminder);
    }

    Ok(Some(values))
}

/// Return whether the current EventKit user declined one native event.
pub(super) fn is_declined_for_current_user_native(event: &EKEvent) -> bool {
    let Some(attendees) = (unsafe { event.attendees() }) else {
        return false;
    };

    attendees.iter().any(|attendee| {
        let is_current_user = unsafe { attendee.isCurrentUser() };
        let participant_status = unsafe { attendee.participantStatus() };

        is_current_user && participant_status == EKParticipantStatus::Declined
    })
}
