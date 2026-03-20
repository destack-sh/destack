use windows::ApplicationModel::Appointments::{
    Appointment, AppointmentBusyStatus, AppointmentDaysOfWeek, AppointmentParticipantResponse,
    AppointmentRecurrence, AppointmentRecurrenceUnit, AppointmentWeekOfMonth,
};
use windows::Foundation::{IReference, PropertyValue, TimeSpan};
use windows::core::Interface;

use super::core::{
    datetime_from_unix_ns, timespan_from_ns, unix_ns_from_datetime, windows_calendar_error,
};
use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarRecurrenceFrequency, CalendarRecurrenceRuleValue, CalendarRecurrenceWeekday,
    CalendarRelativeReminderValue, CalendarReminderValue,
};
use crate::platform::os::{CalendarAvailability, CalendarParticipantStatus};

/// Apply one optional recurrence rule to one native appointment.
pub(super) fn apply_recurrence_rule(
    appointment: &Appointment,
    recurrence_rule: Option<&CalendarRecurrenceRuleValue>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(recurrence_rule) = recurrence_rule else {
        return Ok(());
    };

    let native_recurrence = AppointmentRecurrence::new()
        .map_err(|error| windows_calendar_error(operation, "AppointmentRecurrence::new", &error))?;

    native_recurrence
        .SetUnit(recurrence_unit_from_value(recurrence_rule))
        .map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::SetUnit", &error)
        })?;
    native_recurrence
        .SetInterval(recurrence_rule.interval)
        .map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::SetInterval", &error)
        })?;

    if let Some(count) = recurrence_rule.count {
        let count = PropertyValue::CreateUInt32(count).map_err(|error| {
            windows_calendar_error(operation, "PropertyValue::CreateUInt32", &error)
        })?;
        let count: IReference<u32> = count.cast().map_err(|error| {
            windows_calendar_error(operation, "IInspectable::cast<IReference<u32>>", &error)
        })?;

        native_recurrence.SetOccurrences(&count).map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::SetOccurrences", &error)
        })?;
    }

    if let Some(until_unix_ns) = recurrence_rule.until_unix_ns {
        let until = PropertyValue::CreateDateTime(datetime_from_unix_ns(until_unix_ns)).map_err(
            |error| windows_calendar_error(operation, "PropertyValue::CreateDateTime", &error),
        )?;
        let until: IReference<windows::Foundation::DateTime> = until.cast().map_err(|error| {
            windows_calendar_error(
                operation,
                "IInspectable::cast<IReference<DateTime>>",
                &error,
            )
        })?;

        native_recurrence.SetUntil(&until).map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::SetUntil", &error)
        })?;
    }

    let native_days = appointment_days_of_week(recurrence_rule.by_week_days.as_slice());
    native_recurrence
        .SetDaysOfWeek(native_days)
        .map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::SetDaysOfWeek", &error)
        })?;

    if let Some(weekday) = recurrence_rule
        .by_weekday_ordinals
        .iter()
        .max_by_key(|weekday| weekday.week_number.map(std::cmp::Reverse))
        && let Some(week_number) = weekday.week_number
    {
        native_recurrence
            .SetWeekOfMonth(appointment_week_of_month(week_number))
            .map_err(|error| {
                windows_calendar_error(operation, "AppointmentRecurrence::SetWeekOfMonth", &error)
            })?;
    }

    if let Some(month) = recurrence_rule.by_months.first() {
        native_recurrence.SetMonth(*month as u32).map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::SetMonth", &error)
        })?;
    }

    appointment
        .SetRecurrence(&native_recurrence)
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetRecurrence", &error))?;

    Ok(())
}

/// Apply one optional reminder list to one native appointment.
pub(super) fn apply_reminders(
    appointment: &Appointment,
    reminders: Option<&[CalendarReminderValue]>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(reminders) = reminders else {
        return Ok(());
    };

    let Some(reminder) = reminders.first() else {
        return Ok(());
    };

    let CalendarReminderValue::CalendarRelativeReminder(reminder) = reminder else {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "windows calendar only supports relative reminders",
        ));
    };

    // WinRT stores reminder lead time as one positive duration before start
    if reminder.minutes_before_start < 0 {
        return Err(invalid_argument_value(
            "reminder.minutes_before_start",
            "windows calendar does not support reminders after the event start",
        ));
    }

    let lead_time_ns = i64::from(reminder.minutes_before_start)
        .saturating_mul(60)
        .saturating_mul(1_000_000_000);
    let reminder =
        PropertyValue::CreateTimeSpan(timespan_from_ns(lead_time_ns)).map_err(|error| {
            windows_calendar_error(operation, "PropertyValue::CreateTimeSpan", &error)
        })?;
    let reminder: IReference<TimeSpan> = reminder.cast().map_err(|error| {
        windows_calendar_error(
            operation,
            "IInspectable::cast<IReference<TimeSpan>>",
            &error,
        )
    })?;

    appointment
        .SetReminder(&reminder)
        .map_err(|error| windows_calendar_error(operation, "Appointment::SetReminder", &error))?;

    Ok(())
}

/// Return one runtime recurrence rule when the native appointment is recurring.
pub(super) fn recurrence_rule_from_native(
    appointment: &Appointment,
    operation: &'static str,
) -> RuntimeResult<Option<CalendarRecurrenceRuleValue>> {
    let recurrence = appointment
        .Recurrence()
        .map_err(|error| windows_calendar_error(operation, "Appointment::Recurrence", &error))?;
    let unit = recurrence.Unit().map_err(|error| {
        windows_calendar_error(operation, "AppointmentRecurrence::Unit", &error)
    })?;

    if !is_supported_recurrence_unit(unit) {
        return Ok(None);
    }

    let occurrences = recurrence
        .Occurrences()
        .ok()
        .and_then(|value| value.Value().ok());
    let until_unix_ns = recurrence
        .Until()
        .ok()
        .and_then(|value| value.Value().ok())
        .map(unix_ns_from_datetime);
    let days_of_week = recurrence.DaysOfWeek().map_err(|error| {
        windows_calendar_error(operation, "AppointmentRecurrence::DaysOfWeek", &error)
    })?;
    let week_of_month = recurrence.WeekOfMonth().map_err(|error| {
        windows_calendar_error(operation, "AppointmentRecurrence::WeekOfMonth", &error)
    })?;
    let month = recurrence.Month().map_err(|error| {
        windows_calendar_error(operation, "AppointmentRecurrence::Month", &error)
    })?;

    Ok(Some(CalendarRecurrenceRuleValue {
        frequency: recurrence_frequency_from_unit(unit),
        interval: recurrence.Interval().map_err(|error| {
            windows_calendar_error(operation, "AppointmentRecurrence::Interval", &error)
        })?,
        count: occurrences,
        until_unix_ns,
        by_week_days: calendar_week_days(days_of_week),
        by_weekday_ordinals: calendar_weekday_ordinals(days_of_week, week_of_month),
        by_month_days: Vec::new(),
        by_months: if month == 0 {
            Vec::new()
        } else {
            vec![month as u8]
        },
        by_year_days: Vec::new(),
        by_week_numbers: Vec::new(),
        by_set_positions: Vec::new(),
    }))
}

/// Return one runtime reminder list from the native appointment.
pub(super) fn reminders_from_native(
    appointment: &Appointment,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<CalendarReminderValue>>> {
    let reminder = appointment
        .Reminder()
        .map_err(|error| windows_calendar_error(operation, "Appointment::Reminder", &error))?;
    let reminder = reminder.Value().map_err(|error| {
        windows_calendar_error(operation, "IReference<TimeSpan>::Value", &error)
    })?;
    let lead_time_ns = super::core::duration_ns_from_timespan(reminder);
    let minutes_before_start = (lead_time_ns / 60_000_000_000) as i32;

    Ok(Some(vec![CalendarReminderValue::CalendarRelativeReminder(
        CalendarRelativeReminderValue {
            kind: "relative".to_string(),
            minutes_before_start,
        },
    )]))
}

/// Return the runtime availability for one native busy status.
pub(super) fn availability_from_busy_status(status: AppointmentBusyStatus) -> CalendarAvailability {
    match status {
        AppointmentBusyStatus::Busy => CalendarAvailability::Busy,
        AppointmentBusyStatus::Tentative => CalendarAvailability::Tentative,
        AppointmentBusyStatus::Free => CalendarAvailability::Free,
        AppointmentBusyStatus::OutOfOffice => CalendarAvailability::OutOfOffice,
        AppointmentBusyStatus::WorkingElsewhere => CalendarAvailability::Unavailable,
        _ => CalendarAvailability::Unknown,
    }
}

/// Return the native busy status for one runtime availability.
pub(super) fn busy_status_from_availability(
    availability: CalendarAvailability,
) -> AppointmentBusyStatus {
    match availability {
        CalendarAvailability::Busy => AppointmentBusyStatus::Busy,
        CalendarAvailability::Tentative => AppointmentBusyStatus::Tentative,
        CalendarAvailability::Free => AppointmentBusyStatus::Free,
        CalendarAvailability::OutOfOffice => AppointmentBusyStatus::OutOfOffice,
        CalendarAvailability::Unavailable => AppointmentBusyStatus::WorkingElsewhere,
        CalendarAvailability::Unknown => AppointmentBusyStatus::Busy,
    }
}

/// Return the runtime participant status for one native response.
pub(super) fn participant_status_from_response(
    response: AppointmentParticipantResponse,
) -> CalendarParticipantStatus {
    match response {
        AppointmentParticipantResponse::Tentative => CalendarParticipantStatus::Tentative,
        AppointmentParticipantResponse::Accepted => CalendarParticipantStatus::Accepted,
        AppointmentParticipantResponse::Declined => CalendarParticipantStatus::Declined,
        AppointmentParticipantResponse::None => CalendarParticipantStatus::Pending,
        AppointmentParticipantResponse::Unknown => CalendarParticipantStatus::Unknown,
        _ => CalendarParticipantStatus::Unknown,
    }
}

/// Return the native participant response for one runtime status.
pub(super) fn participant_response_from_value(
    response_status: CalendarParticipantStatus,
) -> AppointmentParticipantResponse {
    match response_status {
        CalendarParticipantStatus::Tentative => AppointmentParticipantResponse::Tentative,
        CalendarParticipantStatus::Accepted => AppointmentParticipantResponse::Accepted,
        CalendarParticipantStatus::Declined => AppointmentParticipantResponse::Declined,
        CalendarParticipantStatus::Pending => AppointmentParticipantResponse::None,
        CalendarParticipantStatus::Unknown
        | CalendarParticipantStatus::Delegated
        | CalendarParticipantStatus::Completed
        | CalendarParticipantStatus::InProcess => AppointmentParticipantResponse::Unknown,
    }
}

/// Return whether one native recurrence unit maps to the shared calendar model.
fn is_supported_recurrence_unit(unit: AppointmentRecurrenceUnit) -> bool {
    matches!(
        unit,
        AppointmentRecurrenceUnit::Daily
            | AppointmentRecurrenceUnit::Weekly
            | AppointmentRecurrenceUnit::Monthly
            | AppointmentRecurrenceUnit::MonthlyOnDay
            | AppointmentRecurrenceUnit::Yearly
            | AppointmentRecurrenceUnit::YearlyOnDay
    )
}

/// Return the runtime recurrence frequency for one native recurrence unit.
fn recurrence_frequency_from_unit(unit: AppointmentRecurrenceUnit) -> CalendarRecurrenceFrequency {
    match unit {
        AppointmentRecurrenceUnit::Daily => CalendarRecurrenceFrequency::Daily,
        AppointmentRecurrenceUnit::Weekly => CalendarRecurrenceFrequency::Weekly,
        AppointmentRecurrenceUnit::Monthly | AppointmentRecurrenceUnit::MonthlyOnDay => {
            CalendarRecurrenceFrequency::Monthly
        }
        AppointmentRecurrenceUnit::Yearly | AppointmentRecurrenceUnit::YearlyOnDay => {
            CalendarRecurrenceFrequency::Yearly
        }
        _ => CalendarRecurrenceFrequency::Weekly,
    }
}

/// Return the native recurrence unit for one runtime rule.
fn recurrence_unit_from_value(
    recurrence_rule: &CalendarRecurrenceRuleValue,
) -> AppointmentRecurrenceUnit {
    match recurrence_rule.frequency {
        CalendarRecurrenceFrequency::Daily => AppointmentRecurrenceUnit::Daily,
        CalendarRecurrenceFrequency::Weekly => AppointmentRecurrenceUnit::Weekly,
        CalendarRecurrenceFrequency::Monthly => {
            if recurrence_rule.by_weekday_ordinals.is_empty() {
                AppointmentRecurrenceUnit::Monthly
            } else {
                AppointmentRecurrenceUnit::MonthlyOnDay
            }
        }
        CalendarRecurrenceFrequency::Yearly => {
            if recurrence_rule.by_weekday_ordinals.is_empty() {
                AppointmentRecurrenceUnit::Yearly
            } else {
                AppointmentRecurrenceUnit::YearlyOnDay
            }
        }
    }
}

/// Return runtime weekday numbers for one native day bitset.
fn calendar_week_days(days_of_week: AppointmentDaysOfWeek) -> Vec<u8> {
    let mut days = Vec::new();

    if days_of_week.contains(AppointmentDaysOfWeek::Monday) {
        days.push(1);
    }
    if days_of_week.contains(AppointmentDaysOfWeek::Tuesday) {
        days.push(2);
    }
    if days_of_week.contains(AppointmentDaysOfWeek::Wednesday) {
        days.push(3);
    }
    if days_of_week.contains(AppointmentDaysOfWeek::Thursday) {
        days.push(4);
    }
    if days_of_week.contains(AppointmentDaysOfWeek::Friday) {
        days.push(5);
    }
    if days_of_week.contains(AppointmentDaysOfWeek::Saturday) {
        days.push(6);
    }
    if days_of_week.contains(AppointmentDaysOfWeek::Sunday) {
        days.push(7);
    }

    days
}

/// Return runtime structured weekday selectors for one native rule.
fn calendar_weekday_ordinals(
    days_of_week: AppointmentDaysOfWeek,
    week_of_month: AppointmentWeekOfMonth,
) -> Vec<CalendarRecurrenceWeekday> {
    let week_number = calendar_week_number(week_of_month);

    calendar_week_days(days_of_week)
        .into_iter()
        .map(|day| CalendarRecurrenceWeekday { day, week_number })
        .collect()
}

/// Return the runtime week number for one native week-of-month value.
fn calendar_week_number(week_of_month: AppointmentWeekOfMonth) -> Option<i8> {
    match week_of_month {
        AppointmentWeekOfMonth::First => Some(1),
        AppointmentWeekOfMonth::Second => Some(2),
        AppointmentWeekOfMonth::Third => Some(3),
        AppointmentWeekOfMonth::Fourth => Some(4),
        AppointmentWeekOfMonth::Last => Some(-1),
        _ => None,
    }
}

/// Return the native week-of-month enum for one runtime ordinal.
fn appointment_week_of_month(week_number: i8) -> AppointmentWeekOfMonth {
    match week_number {
        1 => AppointmentWeekOfMonth::First,
        2 => AppointmentWeekOfMonth::Second,
        3 => AppointmentWeekOfMonth::Third,
        4 => AppointmentWeekOfMonth::Fourth,
        -1 => AppointmentWeekOfMonth::Last,
        _ => AppointmentWeekOfMonth::First,
    }
}

/// Return the native day bitset for one runtime weekday list.
fn appointment_days_of_week(days: &[u8]) -> AppointmentDaysOfWeek {
    let mut native_days = AppointmentDaysOfWeek::None;

    for day in days {
        native_days |= match day {
            1 => AppointmentDaysOfWeek::Monday,
            2 => AppointmentDaysOfWeek::Tuesday,
            3 => AppointmentDaysOfWeek::Wednesday,
            4 => AppointmentDaysOfWeek::Thursday,
            5 => AppointmentDaysOfWeek::Friday,
            6 => AppointmentDaysOfWeek::Saturday,
            7 => AppointmentDaysOfWeek::Sunday,
            _ => AppointmentDaysOfWeek::None,
        };
    }

    native_days
}
