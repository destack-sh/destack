use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_event_kit::{
    EKAlarm, EKEvent, EKEventAvailability, EKParticipantStatus, EKRecurrenceDayOfWeek,
    EKRecurrenceEnd, EKRecurrenceFrequency, EKRecurrenceRule, EKWeekday,
};
use objc2_foundation::{NSArray, NSNumber};

use super::core::{
    CALENDAR_EVENT_READ_OPERATION, calendar_invalid_argument, ns_date_from_unix_ns,
    unix_ns_from_date,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    CalendarAbsoluteReminderValue, CalendarAvailability, CalendarParticipantStatus,
    CalendarRecurrenceFrequency, CalendarRecurrenceRuleValue, CalendarRecurrenceWeekday,
    CalendarRelativeReminderValue, CalendarReminderValue,
};

/// Decode one EventKit recurrence rule from the first native recurrence rule slot.
pub(super) fn recurrence_rule_from_native(
    event: &EKEvent,
) -> RuntimeResult<Option<CalendarRecurrenceRuleValue>> {
    let Some(rules) = (unsafe { event.recurrenceRules() }) else {
        return Ok(None);
    };

    if rules.len() > 1 {
        return Err(super::core::calendar_error(
            CALENDAR_EVENT_READ_OPERATION,
            "multiple native recurrence rules cannot be represented by one runtime calendar event",
        ));
    }

    let Some(rule) = rules.iter().next() else {
        return Ok(None);
    };

    let end = unsafe { rule.recurrenceEnd() };
    let count = end
        .as_ref()
        .map(|value| unsafe { value.occurrenceCount() as u32 })
        .filter(|value| *value > 0);
    let until_unix_ns = end
        .as_ref()
        .and_then(|value| unsafe { value.endDate() })
        .map(|value| unix_ns_from_date(value.as_ref()));
    let mut by_week_days = Vec::new();
    let mut by_weekday_ordinals = Vec::new();

    // split simple weekdays from ordinal selectors
    if let Some(days) = unsafe { rule.daysOfTheWeek() } {
        for day in days.iter() {
            let week_number = unsafe { day.weekNumber() };
            let weekday = CalendarRecurrenceWeekday {
                day: iso_weekday_from_native(unsafe { day.dayOfTheWeek() }),
                week_number: (week_number != 0).then_some(week_number as i8),
            };

            if weekday.week_number.is_some() {
                by_weekday_ordinals.push(weekday);
            } else {
                by_week_days.push(weekday.day);
            }
        }
    }

    Ok(Some(CalendarRecurrenceRuleValue {
        frequency: recurrence_frequency_from_native(unsafe { rule.frequency() }),
        interval: unsafe { rule.interval() } as u32,
        count,
        until_unix_ns,
        by_week_days,
        by_weekday_ordinals,
        by_month_days: nsnumber_vec_i8(unsafe { rule.daysOfTheMonth() }),
        by_months: nsnumber_vec_u8(unsafe { rule.monthsOfTheYear() }),
        by_year_days: nsnumber_vec_i16(unsafe { rule.daysOfTheYear() }),
        by_week_numbers: nsnumber_vec_i8(unsafe { rule.weeksOfTheYear() }),
        by_set_positions: nsnumber_vec_i16(unsafe { rule.setPositions() }),
    }))
}

/// Build one runtime reminder from one native alarm.
pub(super) fn reminder_from_native(alarm: &EKAlarm) -> RuntimeResult<CalendarReminderValue> {
    // absolute alarm
    if let Some(absolute_date) = unsafe { alarm.absoluteDate() } {
        return Ok(CalendarReminderValue::CalendarAbsoluteReminder(
            CalendarAbsoluteReminderValue {
                kind: "absolute".to_string(),
                absolute_unix_ns: unix_ns_from_date(absolute_date.as_ref()),
            },
        ));
    }

    // EventKit relative alarms are event-start anchored
    let offset_seconds = unsafe { alarm.relativeOffset() }.round() as i64;

    Ok(CalendarReminderValue::CalendarRelativeReminder(
        CalendarRelativeReminderValue {
            kind: "relative".to_string(),
            minutes_before_start: (-offset_seconds / 60) as i32,
        },
    ))
}

/// Lower one runtime reminder into one native EventKit alarm.
pub(super) fn reminder_to_native(
    reminder: &CalendarReminderValue,
    _operation: &'static str,
) -> RuntimeResult<Retained<EKAlarm>> {
    match reminder {
        CalendarReminderValue::CalendarAbsoluteReminder(value) => {
            let date = ns_date_from_unix_ns(value.absolute_unix_ns);

            Ok(unsafe { EKAlarm::alarmWithAbsoluteDate(date.as_ref()) })
        }
        CalendarReminderValue::CalendarRelativeReminder(value) => {
            // EventKit relative alarms stay start-relative in the current runtime shape
            let offset_seconds = -(value.minutes_before_start as f64) * 60.0;

            Ok(unsafe { EKAlarm::alarmWithRelativeOffset(offset_seconds) })
        }
    }
}

/// Lower one runtime recurrence rule into one native EventKit recurrence rule.
pub(super) fn recurrence_rule_to_native(
    rule: &CalendarRecurrenceRuleValue,
    operation: &'static str,
) -> RuntimeResult<Retained<EKRecurrenceRule>> {
    if rule.interval == 0 {
        return Err(calendar_invalid_argument(
            operation,
            "calendar recurrence interval must be greater than zero",
        ));
    }

    if rule.count.is_some() && rule.until_unix_ns.is_some() {
        return Err(calendar_invalid_argument(
            operation,
            "calendar recurrence cannot set both count and until_unix_ns",
        ));
    }

    let end = recurrence_end_to_native(rule);
    let weekdays = recurrence_days_to_native(rule, operation)?;
    let month_days = nsnumber_array_i8(&rule.by_month_days);
    let months = nsnumber_array_u8(&rule.by_months);
    let week_numbers = nsnumber_array_i8(&rule.by_week_numbers);
    let year_days = nsnumber_array_i16(&rule.by_year_days);
    let set_positions = nsnumber_array_i16(&rule.by_set_positions);

    Ok(unsafe {
        EKRecurrenceRule::initRecurrenceWithFrequency_interval_daysOfTheWeek_daysOfTheMonth_monthsOfTheYear_weeksOfTheYear_daysOfTheYear_setPositions_end(
            EKRecurrenceRule::alloc(),
            recurrence_frequency_to_native(rule.frequency),
            rule.interval as isize,
            weekdays.as_deref(),
            month_days.as_deref(),
            months.as_deref(),
            week_numbers.as_deref(),
            year_days.as_deref(),
            set_positions.as_deref(),
            end.as_deref(),
        )
    })
}

/// Lower one runtime recurrence end into one native EventKit value.
fn recurrence_end_to_native(
    rule: &CalendarRecurrenceRuleValue,
) -> Option<Retained<EKRecurrenceEnd>> {
    if let Some(count) = rule.count {
        return Some(unsafe { EKRecurrenceEnd::recurrenceEndWithOccurrenceCount(count as usize) });
    }

    if let Some(until_unix_ns) = rule.until_unix_ns {
        let date = ns_date_from_unix_ns(until_unix_ns);

        return Some(unsafe { EKRecurrenceEnd::recurrenceEndWithEndDate(date.as_ref()) });
    }

    None
}

/// Lower one runtime recurrence weekday set into one native EventKit array.
fn recurrence_days_to_native(
    rule: &CalendarRecurrenceRuleValue,
    operation: &'static str,
) -> RuntimeResult<Option<Retained<NSArray<EKRecurrenceDayOfWeek>>>> {
    if rule.by_week_days.is_empty() && rule.by_weekday_ordinals.is_empty() {
        return Ok(None);
    }

    let mut days = Vec::with_capacity(rule.by_week_days.len() + rule.by_weekday_ordinals.len());

    // plain weekday selectors
    for weekday in &rule.by_week_days {
        let weekday = native_weekday_from_iso(*weekday, operation)?;
        let day = unsafe { EKRecurrenceDayOfWeek::dayOfWeek(weekday) };

        days.push(day);
    }

    // ordinal weekday selectors
    for weekday in &rule.by_weekday_ordinals {
        let day = native_weekday_from_iso(weekday.day, operation)?;
        let week_number = weekday.week_number.unwrap_or(0) as isize;
        let native_day = unsafe { EKRecurrenceDayOfWeek::dayOfWeek_weekNumber(day, week_number) };

        days.push(native_day);
    }

    Ok(Some(NSArray::from_retained_slice(&days)))
}

/// Build one Foundation number array from signed values.
fn nsnumber_array_i8(values: &[i8]) -> Option<Retained<NSArray<NSNumber>>> {
    if values.is_empty() {
        return None;
    }

    let numbers = values
        .iter()
        .map(|value| NSNumber::new_i32(*value as i32))
        .collect::<Vec<_>>();

    Some(NSArray::from_retained_slice(&numbers))
}

/// Build one Foundation number array from unsigned values.
fn nsnumber_array_u8(values: &[u8]) -> Option<Retained<NSArray<NSNumber>>> {
    if values.is_empty() {
        return None;
    }

    let numbers = values
        .iter()
        .map(|value| NSNumber::new_u32(*value as u32))
        .collect::<Vec<_>>();

    Some(NSArray::from_retained_slice(&numbers))
}

/// Build one Foundation number array from larger signed values.
fn nsnumber_array_i16(values: &[i16]) -> Option<Retained<NSArray<NSNumber>>> {
    if values.is_empty() {
        return None;
    }

    let numbers = values
        .iter()
        .map(|value| NSNumber::new_i32(*value as i32))
        .collect::<Vec<_>>();

    Some(NSArray::from_retained_slice(&numbers))
}

/// Decode one Foundation number array into signed values.
fn nsnumber_vec_i8(values: Option<Retained<NSArray<NSNumber>>>) -> Vec<i8> {
    let Some(values) = values else {
        return Vec::new();
    };

    values.iter().map(|value| value.as_i32() as i8).collect()
}

/// Decode one Foundation number array into unsigned values.
fn nsnumber_vec_u8(values: Option<Retained<NSArray<NSNumber>>>) -> Vec<u8> {
    let Some(values) = values else {
        return Vec::new();
    };

    values.iter().map(|value| value.as_u32() as u8).collect()
}

/// Decode one Foundation number array into larger signed values.
fn nsnumber_vec_i16(values: Option<Retained<NSArray<NSNumber>>>) -> Vec<i16> {
    let Some(values) = values else {
        return Vec::new();
    };

    values.iter().map(|value| value.as_i32() as i16).collect()
}

/// Map one runtime recurrence frequency into EventKit.
fn recurrence_frequency_to_native(frequency: CalendarRecurrenceFrequency) -> EKRecurrenceFrequency {
    match frequency {
        CalendarRecurrenceFrequency::Daily => EKRecurrenceFrequency::Daily,
        CalendarRecurrenceFrequency::Weekly => EKRecurrenceFrequency::Weekly,
        CalendarRecurrenceFrequency::Monthly => EKRecurrenceFrequency::Monthly,
        CalendarRecurrenceFrequency::Yearly => EKRecurrenceFrequency::Yearly,
    }
}

/// Map one EventKit recurrence frequency into the runtime model.
fn recurrence_frequency_from_native(
    frequency: EKRecurrenceFrequency,
) -> CalendarRecurrenceFrequency {
    match frequency {
        EKRecurrenceFrequency::Daily => CalendarRecurrenceFrequency::Daily,
        EKRecurrenceFrequency::Weekly => CalendarRecurrenceFrequency::Weekly,
        EKRecurrenceFrequency::Monthly => CalendarRecurrenceFrequency::Monthly,
        EKRecurrenceFrequency::Yearly => CalendarRecurrenceFrequency::Yearly,
        _ => CalendarRecurrenceFrequency::Daily,
    }
}

/// Map one runtime availability into EventKit.
pub(super) fn calendar_availability_to_native(
    availability: CalendarAvailability,
    operation: &'static str,
) -> RuntimeResult<EKEventAvailability> {
    match availability {
        CalendarAvailability::Busy => Ok(EKEventAvailability::Busy),
        CalendarAvailability::Free => Ok(EKEventAvailability::Free),
        CalendarAvailability::Tentative => Ok(EKEventAvailability::Tentative),
        CalendarAvailability::Unavailable => Ok(EKEventAvailability::Unavailable),
        CalendarAvailability::Unknown => Err(calendar_invalid_argument(
            operation,
            "calendar availability `Unknown` cannot be written to EventKit",
        )),
        CalendarAvailability::OutOfOffice => Err(calendar_invalid_argument(
            operation,
            "calendar availability `OutOfOffice` is not representable in EventKit",
        )),
    }
}

/// Map one EventKit availability into the runtime model.
pub(super) fn calendar_availability_from_native(
    availability: EKEventAvailability,
) -> CalendarAvailability {
    match availability {
        EKEventAvailability::Busy => CalendarAvailability::Busy,
        EKEventAvailability::Free => CalendarAvailability::Free,
        EKEventAvailability::Tentative => CalendarAvailability::Tentative,
        EKEventAvailability::Unavailable => CalendarAvailability::Unavailable,
        EKEventAvailability::NotSupported => CalendarAvailability::Unknown,
        _ => CalendarAvailability::Unknown,
    }
}

/// Map one EventKit participant status into the runtime model.
pub(super) fn participant_status_from_native(
    status: EKParticipantStatus,
) -> CalendarParticipantStatus {
    match status {
        EKParticipantStatus::Unknown => CalendarParticipantStatus::Unknown,
        EKParticipantStatus::Pending => CalendarParticipantStatus::Pending,
        EKParticipantStatus::Accepted => CalendarParticipantStatus::Accepted,
        EKParticipantStatus::Tentative => CalendarParticipantStatus::Tentative,
        EKParticipantStatus::Declined => CalendarParticipantStatus::Declined,
        EKParticipantStatus::Delegated => CalendarParticipantStatus::Delegated,
        EKParticipantStatus::Completed => CalendarParticipantStatus::Completed,
        EKParticipantStatus::InProcess => CalendarParticipantStatus::InProcess,
        _ => CalendarParticipantStatus::Unknown,
    }
}

/// Convert one ISO weekday number into EventKit weekday numbering.
fn native_weekday_from_iso(day: u8, operation: &'static str) -> RuntimeResult<EKWeekday> {
    match day {
        1 => Ok(EKWeekday::Monday),
        2 => Ok(EKWeekday::Tuesday),
        3 => Ok(EKWeekday::Wednesday),
        4 => Ok(EKWeekday::Thursday),
        5 => Ok(EKWeekday::Friday),
        6 => Ok(EKWeekday::Saturday),
        7 => Ok(EKWeekday::Sunday),
        _ => Err(calendar_invalid_argument(
            operation,
            format!("calendar weekday `{day}` must be in ISO range 1..=7"),
        )),
    }
}

/// Convert one EventKit weekday number into ISO weekday numbering.
fn iso_weekday_from_native(day: EKWeekday) -> u8 {
    match day {
        EKWeekday::Sunday => 7,
        EKWeekday::Monday => 1,
        EKWeekday::Tuesday => 2,
        EKWeekday::Wednesday => 3,
        EKWeekday::Thursday => 4,
        EKWeekday::Friday => 5,
        EKWeekday::Saturday => 6,
        _ => 7,
    }
}
