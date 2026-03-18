use std::collections::BTreeMap;

use jiff::tz::TimeZone;
use vobject::{Component, Property};

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::os::abi_generated::{
    CalendarRecurrenceFrequency, CalendarRecurrenceRuleValue, CalendarRecurrenceWeekday,
};

use super::CALENDAR_EVENT_READ_OPERATION;
use super::time::{calendar_time_zone_error, datetime_property, parse_ical_datetime};

/// Decode one recurrence rule payload from one VEVENT component.
pub(super) fn recurrence_rule_from_component(
    event: &Component,
) -> RuntimeResult<Option<CalendarRecurrenceRuleValue>> {
    let Some(property) = event.get_only("RRULE") else {
        return Ok(None);
    };
    let parts = recurrence_parts(&property.value_as_string());

    let frequency =
        recurrence_frequency_from_raw(parts.get("FREQ").map(String::as_str).unwrap_or_default())?;
    let interval = parts
        .get("INTERVAL")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let count = parts
        .get("COUNT")
        .and_then(|value| value.parse::<u32>().ok());
    let until_unix_ns = parts
        .get("UNTIL")
        .map(|value| parse_ical_datetime(value, CALENDAR_EVENT_READ_OPERATION))
        .transpose()?
        .map(|value| {
            value
                .to_zoned(TimeZone::UTC)
                .map_err(calendar_time_zone_error(CALENDAR_EVENT_READ_OPERATION))
        })
        .transpose()?
        .map(|value| value.timestamp().as_nanosecond().max(0) as u64);

    Ok(Some(CalendarRecurrenceRuleValue {
        frequency,
        interval,
        count,
        until_unix_ns,
        by_week_days: recurrence_week_days(parts.get("BYDAY"))?,
        by_weekday_ordinals: recurrence_weekday_ordinals(parts.get("BYDAY"))?,
        by_month_days: recurrence_number_list(parts.get("BYMONTHDAY"))?,
        by_months: recurrence_positive_number_list(parts.get("BYMONTH"))?,
        by_year_days: recurrence_number_list(parts.get("BYYEARDAY"))?,
        by_week_numbers: recurrence_number_list(parts.get("BYWEEKNO"))?,
        by_set_positions: recurrence_number_list(parts.get("BYSETPOS"))?,
    }))
}

/// Encode one recurrence rule as one RRULE property.
pub(super) fn recurrence_rule_property(
    rule: &CalendarRecurrenceRuleValue,
    operation: &'static str,
) -> RuntimeResult<Property> {
    if rule.interval == 0 {
        return Err(invalid_argument_value(
            operation,
            "event",
            "calendar recurrence interval must be greater than zero",
        ));
    }

    if rule.count.is_some() && rule.until_unix_ns.is_some() {
        return Err(invalid_argument_value(
            operation,
            "event",
            "calendar recurrence cannot set both count and until_unix_ns",
        ));
    }

    let mut parts = Vec::new();
    parts.push(format!(
        "FREQ={}",
        match rule.frequency {
            CalendarRecurrenceFrequency::Daily => "DAILY",
            CalendarRecurrenceFrequency::Weekly => "WEEKLY",
            CalendarRecurrenceFrequency::Monthly => "MONTHLY",
            CalendarRecurrenceFrequency::Yearly => "YEARLY",
        }
    ));
    parts.push(format!("INTERVAL={}", rule.interval));

    if let Some(count) = rule.count {
        parts.push(format!("COUNT={count}"));
    }

    if let Some(until_unix_ns) = rule.until_unix_ns {
        let until = datetime_property("UNTIL", until_unix_ns, false, Some("UTC"), operation)?;
        parts.push(format!("UNTIL={}", until.value_as_string()));
    }

    if !rule.by_weekday_ordinals.is_empty() {
        parts.push(format!(
            "BYDAY={}",
            rule.by_weekday_ordinals
                .iter()
                .map(recurrence_weekday_token)
                .collect::<Vec<_>>()
                .join(",")
        ));
    } else if !rule.by_week_days.is_empty() {
        parts.push(format!(
            "BYDAY={}",
            rule.by_week_days
                .iter()
                .map(|day| weekday_token(*day, None))
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    append_rrule_number_part(&mut parts, "BYMONTHDAY", &rule.by_month_days);
    append_rrule_number_part(&mut parts, "BYMONTH", &rule.by_months);
    append_rrule_number_part(&mut parts, "BYYEARDAY", &rule.by_year_days);
    append_rrule_number_part(&mut parts, "BYWEEKNO", &rule.by_week_numbers);
    append_rrule_number_part(&mut parts, "BYSETPOS", &rule.by_set_positions);

    Ok(Property::new("RRULE", parts.join(";")))
}

/// Decode one RRULE string into key-value parts.
fn recurrence_parts(value: &str) -> BTreeMap<String, String> {
    let mut parts = BTreeMap::new();

    for token in value.split(';') {
        let Some((name, raw_value)) = token.split_once('=') else {
            continue;
        };

        parts.insert(name.to_string(), raw_value.to_string());
    }

    parts
}

/// Decode one recurrence frequency token.
fn recurrence_frequency_from_raw(value: &str) -> RuntimeResult<CalendarRecurrenceFrequency> {
    match value {
        "DAILY" => Ok(CalendarRecurrenceFrequency::Daily),
        "WEEKLY" => Ok(CalendarRecurrenceFrequency::Weekly),
        "MONTHLY" => Ok(CalendarRecurrenceFrequency::Monthly),
        "YEARLY" => Ok(CalendarRecurrenceFrequency::Yearly),
        _ => Err(invalid_argument_value(
            CALENDAR_EVENT_READ_OPERATION,
            "calendar",
            format!("unsupported calendar recurrence frequency `{value}`"),
        )),
    }
}

/// Decode one BYDAY token list into simple weekdays.
fn recurrence_week_days(value: Option<&String>) -> RuntimeResult<Vec<u8>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    value
        .split(',')
        .map(|token| weekday_from_token(token).map(|value| value.day))
        .collect()
}

/// Decode one BYDAY token list into weekday ordinals.
fn recurrence_weekday_ordinals(
    value: Option<&String>,
) -> RuntimeResult<Vec<CalendarRecurrenceWeekday>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    value.split(',').map(weekday_from_token).collect()
}

/// Decode one RRULE numeric list.
fn recurrence_number_list<T>(value: Option<&String>) -> RuntimeResult<Vec<T>>
where
    T: std::str::FromStr,
{
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    value
        .split(',')
        .map(|token| {
            token.parse::<T>().map_err(|_| {
                invalid_argument_value(
                    CALENDAR_EVENT_READ_OPERATION,
                    "calendar",
                    format!("invalid calendar recurrence number `{token}`"),
                )
            })
        })
        .collect()
}

/// Decode one RRULE positive numeric list.
fn recurrence_positive_number_list<T>(value: Option<&String>) -> RuntimeResult<Vec<T>>
where
    T: std::str::FromStr,
{
    recurrence_number_list(value)
}

/// Decode one BYDAY token into one weekday payload.
fn weekday_from_token(token: &str) -> RuntimeResult<CalendarRecurrenceWeekday> {
    let token = token.trim();
    if token.len() < 2 {
        return Err(invalid_argument_value(
            CALENDAR_EVENT_READ_OPERATION,
            "calendar",
            format!("invalid calendar weekday token `{token}`"),
        ));
    }

    let (prefix, day_token) = token.split_at(token.len() - 2);
    let day = match day_token {
        "MO" => 1,
        "TU" => 2,
        "WE" => 3,
        "TH" => 4,
        "FR" => 5,
        "SA" => 6,
        "SU" => 7,
        _ => {
            return Err(invalid_argument_value(
                CALENDAR_EVENT_READ_OPERATION,
                "calendar",
                format!("invalid calendar weekday token `{token}`"),
            ));
        }
    };
    let week_number = if prefix.is_empty() {
        None
    } else {
        Some(prefix.parse::<i8>().map_err(|_| {
            invalid_argument_value(
                CALENDAR_EVENT_READ_OPERATION,
                "calendar",
                format!("invalid calendar weekday ordinal `{token}`"),
            )
        })?)
    };

    Ok(CalendarRecurrenceWeekday { day, week_number })
}

/// Encode one weekday ordinal token.
fn recurrence_weekday_token(weekday: &CalendarRecurrenceWeekday) -> String {
    weekday_token(weekday.day, weekday.week_number)
}

/// Encode one weekday token.
fn weekday_token(day: u8, week_number: Option<i8>) -> String {
    let day_token = match day {
        1 => "MO",
        2 => "TU",
        3 => "WE",
        4 => "TH",
        5 => "FR",
        6 => "SA",
        7 => "SU",
        _ => "MO",
    };

    if let Some(week_number) = week_number {
        return format!("{week_number}{day_token}");
    }

    day_token.to_string()
}

/// Append one numeric RRULE part when populated.
fn append_rrule_number_part<T>(parts: &mut Vec<String>, name: &str, values: &[T])
where
    T: ToString,
{
    if values.is_empty() {
        return;
    }

    parts.push(format!(
        "{name}={}",
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ));
}

#[cfg(test)]
mod tests {
    use vobject::{Component, Property};

    use super::recurrence_rule_from_component;
    use crate::platform::os::CalendarRecurrenceFrequency;

    #[test]
    fn test_recurrence_rule_from_component_decodes_until_utc() {
        let mut event = Component::new("VEVENT");
        event.set(Property::new("RRULE", "FREQ=DAILY;UNTIL=20260415T090000Z"));
        let rule = recurrence_rule_from_component(&event).unwrap().unwrap();

        assert_eq!(rule.frequency, CalendarRecurrenceFrequency::Daily);
        assert!(rule.until_unix_ns.is_some());
    }
}
