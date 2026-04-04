use vobject::{Component, Property};

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::os::CalendarReminderAnchor;
use crate::platform::os::abi_generated::{
    CalendarAbsoluteReminderValue, CalendarRelativeReminderValue, CalendarReminderValue,
};

use super::CALENDAR_EVENT_READ_OPERATION;
use super::time::{datetime_property, datetime_value_from_property, duration_seconds_from_ical};

/// Decode one reminder list from one VCALENDAR payload.
pub(super) fn reminders_from_component(
    calendar: &Component,
) -> RuntimeResult<Option<Vec<CalendarReminderValue>>> {
    let mut reminders = Vec::new();

    // valarm materialization
    for subcomponent in &calendar.subcomponents {
        if subcomponent.name != "VALARM" {
            continue;
        }

        let Some(trigger) = subcomponent.get_only("TRIGGER") else {
            continue;
        };

        let reminder = reminder_from_trigger(trigger)?;
        reminders.push(reminder);
    }

    if reminders.is_empty() {
        return Ok(None);
    }

    Ok(Some(reminders))
}

/// Decode one reminder from one iCalendar trigger property.
pub(super) fn reminder_from_trigger(property: &Property) -> RuntimeResult<CalendarReminderValue> {
    let value = property.value_as_string();

    // absolute trigger
    if !value.starts_with('P') && !value.starts_with("+P") && !value.starts_with("-P") {
        let value = datetime_value_from_property(property, CALENDAR_EVENT_READ_OPERATION)?;

        return Ok(CalendarReminderValue::CalendarAbsoluteReminder(
            CalendarAbsoluteReminderValue {
                kind: "absolute".to_string(),
                absolute_unix_ns: value.unix_ns,
            },
        ));
    }

    // trigger anchor
    let related = property
        .params
        .get("RELATED")
        .map(String::as_str)
        .unwrap_or("START");
    let anchor = reminder_anchor_from_related(related)?;

    // signed offset
    let offset_seconds = duration_seconds_from_ical(&value, CALENDAR_EVENT_READ_OPERATION)?;

    Ok(CalendarReminderValue::CalendarRelativeReminder(
        CalendarRelativeReminderValue {
            kind: "relative".to_string(),
            anchor,
            offset_seconds,
        },
    ))
}

/// Encode one reminder as one VALARM component.
pub(super) fn reminder_component(
    reminder: &CalendarReminderValue,
    operation: &'static str,
) -> RuntimeResult<Component> {
    let mut component = Component::new("VALARM");
    component.set(Property::new("ACTION", "DISPLAY"));
    component.set(Property::new("DESCRIPTION", "Reminder"));

    match reminder {
        CalendarReminderValue::CalendarRelativeReminder(value) => {
            let mut trigger = Property::new(
                "TRIGGER",
                ical_duration_from_seconds(value.offset_seconds, operation)?,
            );

            // non-default anchor
            if value.anchor == CalendarReminderAnchor::End {
                trigger
                    .params
                    .insert("RELATED".to_string(), "END".to_string());
            }

            component.set(trigger);
        }
        CalendarReminderValue::CalendarAbsoluteReminder(value) => {
            component.set(datetime_property(
                "TRIGGER",
                value.absolute_unix_ns,
                false,
                Some("UTC"),
                operation,
            )?);
        }
    }

    Ok(component)
}

/// Decode one iCalendar RELATED parameter into one runtime anchor.
fn reminder_anchor_from_related(related: &str) -> RuntimeResult<CalendarReminderAnchor> {
    // default start anchor
    if related.eq_ignore_ascii_case("START") {
        return Ok(CalendarReminderAnchor::Start);
    }

    // end-relative trigger
    if related.eq_ignore_ascii_case("END") {
        return Ok(CalendarReminderAnchor::End);
    }

    Err(invalid_argument_value(
        CALENDAR_EVENT_READ_OPERATION,
        "calendar",
        format!("unsupported iCalendar reminder RELATED value `{related}`"),
    ))
}

/// Encode one signed second offset as one RFC5545 duration payload.
fn ical_duration_from_seconds(value: i64, operation: &'static str) -> RuntimeResult<String> {
    let sign = if value < 0 { "-" } else { "" };
    let total_seconds = value.unsigned_abs();
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;
    let mut duration = String::from(sign);

    // zero duration
    if total_seconds == 0 {
        duration.push_str("PT0S");

        return Ok(duration);
    }

    duration.push('P');

    // date fields
    if days > 0 {
        duration.push_str(&format!("{days}D"));
    }

    // time fields
    if hours > 0 || minutes > 0 || seconds > 0 {
        duration.push('T');
    }

    if hours > 0 {
        duration.push_str(&format!("{hours}H"));
    }

    if minutes > 0 {
        duration.push_str(&format!("{minutes}M"));
    }

    if seconds > 0 {
        duration.push_str(&format!("{seconds}S"));
    }

    if duration == "P" || duration == "-P" {
        return Err(invalid_argument_value(
            operation,
            "calendar",
            format!("invalid calendar reminder offset {value}"),
        ));
    }

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use jiff::tz;
    use vobject::Property;

    use super::{ical_duration_from_seconds, reminder_from_trigger};
    use crate::host::os::unix::request::calendar::linux::time::parse_ical_datetime;
    use crate::platform::os::CalendarReminderAnchor;
    use crate::platform::os::abi_generated::{
        CalendarAbsoluteReminderValue, CalendarRelativeReminderValue, CalendarReminderValue,
    };

    #[test]
    fn test_reminder_from_trigger_decodes_relative_duration() {
        let property = Property::new("TRIGGER", "-PT15M");
        let reminder = reminder_from_trigger(&property).unwrap();

        assert_eq!(
            reminder,
            CalendarReminderValue::CalendarRelativeReminder(CalendarRelativeReminderValue {
                kind: "relative".to_string(),
                anchor: CalendarReminderAnchor::Start,
                offset_seconds: -900,
            }),
        );
    }

    #[test]
    fn test_reminder_from_trigger_decodes_end_relative_duration() {
        let mut property = Property::new("TRIGGER", "PT45S");
        property
            .params
            .insert("RELATED".to_string(), "END".to_string());
        let reminder = reminder_from_trigger(&property).unwrap();

        assert_eq!(
            reminder,
            CalendarReminderValue::CalendarRelativeReminder(CalendarRelativeReminderValue {
                kind: "relative".to_string(),
                anchor: CalendarReminderAnchor::End,
                offset_seconds: 45,
            }),
        );
    }

    #[test]
    fn test_reminder_from_trigger_decodes_absolute_datetime() {
        let mut property = Property::new("TRIGGER", "20260415T090000");
        property
            .params
            .insert("TZID".to_string(), "Europe/Zurich".to_string());
        let reminder = reminder_from_trigger(&property).unwrap();
        let expected_unix_ns =
            parse_ical_datetime("20260415T090000", "destack.os.calendar.eventRead")
                .unwrap()
                .to_zoned(tz::db().get("Europe/Zurich").unwrap())
                .unwrap()
                .timestamp()
                .as_nanosecond()
                .max(0) as u64;

        assert_eq!(
            reminder,
            CalendarReminderValue::CalendarAbsoluteReminder(CalendarAbsoluteReminderValue {
                kind: "absolute".to_string(),
                absolute_unix_ns: expected_unix_ns,
            }),
        );
    }

    #[test]
    fn test_ical_duration_from_seconds_encodes_signed_second_precision() {
        let value = ical_duration_from_seconds(-3_661, "destack.os.calendar.eventCreate").unwrap();

        assert_eq!(value, "-PT1H1M1S");
    }
}
