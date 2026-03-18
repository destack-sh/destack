use jiff::Timestamp;
use jiff::civil::{DateTime, date, time};
use jiff::tz::{self, TimeZone};
use vobject::Property;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::error::invalid_argument_value;
use crate::platform::PlatformError;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;

/// One decoded iCalendar datetime payload.
pub(super) struct CalendarDateTimeValue {
    /// The UTC timestamp in Unix nanoseconds.
    pub(super) unix_ns: u64,
    /// Whether the original property was all-day.
    pub(super) is_all_day: bool,
    /// The original timezone identifier when present.
    pub(super) time_zone: Option<String>,
}

/// Decode one RFC5545 date or datetime property into runtime nanoseconds.
pub(super) fn datetime_value_from_property(
    property: &Property,
    operation: &'static str,
) -> RuntimeResult<CalendarDateTimeValue> {
    let is_all_day = property
        .params
        .get("VALUE")
        .is_some_and(|value| value.eq_ignore_ascii_case("DATE"))
        || property.raw_value.len() == 8;
    let time_zone_name = property.params.get("TZID").cloned();

    // all-day
    if is_all_day {
        let civil = parse_ical_date(&property.raw_value, operation)?;
        let timestamp = civil
            .to_zoned(TimeZone::UTC)
            .map_err(calendar_time_zone_error(operation))?
            .timestamp()
            .as_nanosecond();

        return Ok(CalendarDateTimeValue {
            unix_ns: timestamp.max(0) as u64,
            is_all_day: true,
            time_zone: None,
        });
    }

    let civil = parse_ical_datetime(&property.raw_value, operation)?;

    // UTC timestamp
    if property.raw_value.ends_with('Z') {
        let timestamp = civil
            .to_zoned(TimeZone::UTC)
            .map_err(calendar_time_zone_error(operation))?
            .timestamp()
            .as_nanosecond();

        return Ok(CalendarDateTimeValue {
            unix_ns: timestamp.max(0) as u64,
            is_all_day: false,
            time_zone: Some("UTC".to_string()),
        });
    }

    // named time zone
    if let Some(time_zone_name) = &time_zone_name {
        let time_zone = time_zone_from_name(time_zone_name, operation)?;
        let timestamp = civil
            .to_zoned(time_zone)
            .map_err(calendar_time_zone_error(operation))?
            .timestamp()
            .as_nanosecond();

        return Ok(CalendarDateTimeValue {
            unix_ns: timestamp.max(0) as u64,
            is_all_day: false,
            time_zone: Some(time_zone_name.clone()),
        });
    }

    // floating local time: preserve the civil wall-clock value as UTC
    let timestamp = civil
        .to_zoned(TimeZone::UTC)
        .map_err(calendar_time_zone_error(operation))?
        .timestamp()
        .as_nanosecond();

    Ok(CalendarDateTimeValue {
        unix_ns: timestamp.max(0) as u64,
        is_all_day: false,
        time_zone: None,
    })
}

/// Parse one iCalendar date payload.
fn parse_ical_date(value: &str, operation: &'static str) -> RuntimeResult<DateTime> {
    if value.len() != 8 {
        return Err(invalid_argument_value(
            operation,
            "calendar",
            format!("invalid iCalendar date value `{value}`"),
        ));
    }

    let year = parse_u32(&value[0..4], operation, "year")? as i16;
    let month = parse_u32(&value[4..6], operation, "month")? as i8;
    let day = parse_u32(&value[6..8], operation, "day")? as i8;

    Ok(DateTime::from_parts(
        date(year, month, day),
        time(0, 0, 0, 0),
    ))
}

/// Parse one iCalendar datetime payload.
pub(super) fn parse_ical_datetime(value: &str, operation: &'static str) -> RuntimeResult<DateTime> {
    let value = value.strip_suffix('Z').unwrap_or(value);
    if value.len() != 15 || value.as_bytes()[8] != b'T' {
        return Err(invalid_argument_value(
            operation,
            "calendar",
            format!("invalid iCalendar datetime value `{value}`"),
        ));
    }

    let year = parse_u32(&value[0..4], operation, "year")? as i16;
    let month = parse_u32(&value[4..6], operation, "month")? as i8;
    let day = parse_u32(&value[6..8], operation, "day")? as i8;
    let hour = parse_u32(&value[9..11], operation, "hour")? as i8;
    let minute = parse_u32(&value[11..13], operation, "minute")? as i8;
    let second = parse_u32(&value[13..15], operation, "second")? as i8;

    Ok(DateTime::from_parts(
        date(year, month, day),
        time(hour, minute, second, 0),
    ))
}

/// Parse one iCalendar duration into whole seconds.
pub(super) fn duration_seconds_from_ical(
    value: &str,
    operation: &'static str,
) -> RuntimeResult<i64> {
    let (sign, value) = if let Some(value) = value.strip_prefix('-') {
        (-1i64, value)
    } else if let Some(value) = value.strip_prefix('+') {
        (1i64, value)
    } else {
        (1i64, value)
    };
    let Some(value) = value.strip_prefix('P') else {
        return Err(invalid_argument_value(
            operation,
            "calendar",
            format!("invalid iCalendar duration value `{value}`"),
        ));
    };
    let mut date_part = value;
    let mut time_part = "";

    if let Some((left, right)) = value.split_once('T') {
        date_part = left;
        time_part = right;
    }

    let weeks = duration_field_seconds(date_part, 'W', 7 * 24 * 60 * 60, operation)?;
    let days = duration_field_seconds(date_part, 'D', 24 * 60 * 60, operation)?;
    let hours = duration_field_seconds(time_part, 'H', 60 * 60, operation)?;
    let minutes = duration_field_seconds(time_part, 'M', 60, operation)?;
    let seconds = duration_field_seconds(time_part, 'S', 1, operation)?;
    let total_seconds = weeks
        .checked_add(days)
        .and_then(|value| value.checked_add(hours))
        .and_then(|value| value.checked_add(minutes))
        .and_then(|value| value.checked_add(seconds))
        .ok_or_else(|| {
            invalid_argument_value(
                operation,
                "calendar",
                format!("invalid iCalendar duration value `{value}`"),
            )
        })?;

    Ok(sign.saturating_mul(total_seconds))
}

/// Parse one iCalendar duration field into scaled seconds.
fn duration_field_seconds(
    value: &str,
    suffix: char,
    scale_seconds: i64,
    operation: &'static str,
) -> RuntimeResult<i64> {
    let Some(raw_number) = duration_field_value(value, suffix) else {
        return Ok(0);
    };
    let number = raw_number.parse::<i64>().map_err(|_| {
        invalid_argument_value(
            operation,
            "calendar",
            format!("invalid iCalendar duration value `{value}`"),
        )
    })?;

    number.checked_mul(scale_seconds).ok_or_else(|| {
        invalid_argument_value(
            operation,
            "calendar",
            format!("invalid iCalendar duration value `{value}`"),
        )
    })
}

/// Return one raw iCalendar duration field payload.
fn duration_field_value<'a>(value: &'a str, suffix: char) -> Option<&'a str> {
    let end = value.find(suffix)?;
    let prefix = &value[..end];
    let start = prefix
        .rfind(|character: char| !character.is_ascii_digit())
        .map_or(0, |index| index + 1);

    Some(&prefix[start..])
}

/// Encode one runtime timestamp as one iCalendar datetime property.
pub(super) fn datetime_property(
    name: &str,
    unix_ns: u64,
    is_all_day: bool,
    time_zone_name: Option<&str>,
    operation: &'static str,
) -> RuntimeResult<Property> {
    let timestamp = Timestamp::from_nanosecond(i128::from(unix_ns)).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "event",
            format!("calendar timestamp is invalid: {error}"),
        ))
        .boxed()
    })?;

    // all-day
    if is_all_day {
        let zoned = timestamp.to_zoned(TimeZone::UTC);
        let mut property = Property::new(
            name,
            format!("{:04}{:02}{:02}", zoned.year(), zoned.month(), zoned.day()),
        );
        property
            .params
            .insert("VALUE".to_string(), "DATE".to_string());

        return Ok(property);
    }

    // named time zone
    if let Some(time_zone_name) = time_zone_name {
        if !time_zone_name.is_empty() {
            let time_zone = time_zone_from_name(time_zone_name, operation)?;
            let zoned = timestamp.to_zoned(time_zone);
            let mut property = Property::new(
                name,
                format!(
                    "{:04}{:02}{:02}T{:02}{:02}{:02}",
                    zoned.year(),
                    zoned.month(),
                    zoned.day(),
                    zoned.hour(),
                    zoned.minute(),
                    zoned.second()
                ),
            );
            property
                .params
                .insert("TZID".to_string(), time_zone_name.to_string());

            return Ok(property);
        }
    }

    // UTC fallback
    let zoned = timestamp.to_zoned(TimeZone::UTC);
    Ok(Property::new(
        name,
        format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            zoned.year(),
            zoned.month(),
            zoned.day(),
            zoned.hour(),
            zoned.minute(),
            zoned.second()
        ),
    ))
}

/// Resolve one named time zone.
pub(super) fn time_zone_from_name(
    time_zone_name: &str,
    operation: &'static str,
) -> RuntimeResult<TimeZone> {
    let time_zone_name = time_zone_name.trim();
    if time_zone_name.is_empty() {
        return Ok(TimeZone::UTC);
    }

    if time_zone_name.eq_ignore_ascii_case("utc")
        || time_zone_name.eq_ignore_ascii_case("gmt")
        || time_zone_name == "Z"
    {
        return Ok(TimeZone::UTC);
    }

    tz::db().get(time_zone_name).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "event",
            format!("calendar event time_zone must be one valid timezone identifier: {error}"),
        ))
        .boxed()
    })
}

/// Map one zoned-time failure into one runtime error.
pub(super) fn calendar_time_zone_error(
    operation: &'static str,
) -> impl FnOnce(jiff::Error) -> Box<RuntimeError> {
    move |error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("calendar time conversion failed: {error}"),
        )
    }
}

/// Parse one decimal substring.
fn parse_u32(value: &str, operation: &'static str, field_name: &str) -> RuntimeResult<u32> {
    value.parse::<u32>().map_err(|_| {
        invalid_argument_value(
            operation,
            "calendar",
            format!("invalid calendar {field_name} value `{value}`"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::duration_seconds_from_ical;

    #[test]
    fn test_duration_seconds_from_ical_supports_days_hours_minutes() {
        let seconds =
            duration_seconds_from_ical("-P1DT2H30M", "destack.os.calendar.eventRead").unwrap();

        assert_eq!(seconds, -(24 * 60 * 60 + 2 * 60 * 60 + 30 * 60));
    }
}
