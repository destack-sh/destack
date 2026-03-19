use jiff::civil::DateTime;
use jiff::tz::{self, TimeZone};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationCalendarDateTriggerValue, NotificationCalendarTriggerValue,
};

/// Return the wall-clock delivery timestamp for one calendar-date trigger.
pub(super) fn calendar_date_trigger_unix_ns(
    value: &NotificationCalendarDateTriggerValue,
) -> RuntimeResult<u64> {
    calendar_trigger_unix_ns(&value.calendar)
}

/// Return the wall-clock delivery timestamp for one calendar trigger.
pub(super) fn calendar_trigger_unix_ns(
    value: &NotificationCalendarTriggerValue,
) -> RuntimeResult<u64> {
    let time_zone = notification_time_zone(&value.time_zone)?;
    let year = i16::try_from(value.year).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "trigger",
            format!(
                "calendar notification year `{}` is outside the supported range",
                value.year
            ),
        ))
        .boxed()
    })?;
    let date_time = DateTime::new(
        year,
        value.month as i8,
        value.day as i8,
        value.hour as i8,
        value.minute as i8,
        value.second as i8,
        0,
    )
    .map_err(notification_calendar_error)?;
    let zoned = date_time
        .to_zoned(time_zone)
        .map_err(notification_calendar_error)?;
    let unix_ns = zoned.timestamp().as_nanosecond();

    if unix_ns < 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "trigger",
            "calendar notification trigger must not predate the Unix epoch",
        ))
        .boxed());
    }

    if unix_ns > i128::from(u64::MAX) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "trigger",
            "calendar notification trigger exceeds the supported Unix timestamp range",
        ))
        .boxed());
    }

    Ok(unix_ns as u64)
}

/// Return one resolved timezone for one notification calendar trigger.
pub(super) fn notification_time_zone(time_zone: &str) -> RuntimeResult<TimeZone> {
    let time_zone = time_zone.trim();

    // empty timezone means host local time
    if time_zone.is_empty() {
        return TimeZone::try_system().map_err(notification_system_time_zone_error);
    }

    // canonical utc aliases
    if time_zone.eq_ignore_ascii_case("utc")
        || time_zone.eq_ignore_ascii_case("gmt")
        || time_zone == "Z"
    {
        return Ok(TimeZone::UTC);
    }

    tz::db()
        .get(time_zone)
        .map_err(notification_named_time_zone_error)
}

/// Return one calendar timezone suffix for one systemd expression when needed.
#[cfg(target_os = "linux")]
pub(super) fn notification_time_zone_suffix(time_zone: &str) -> RuntimeResult<Option<String>> {
    let time_zone = time_zone.trim();

    // validate the effective timezone even when systemd will use the implicit local zone
    let _time_zone = notification_time_zone(time_zone)?;

    if time_zone.is_empty() {
        return Ok(None);
    }

    if time_zone.eq_ignore_ascii_case("utc")
        || time_zone.eq_ignore_ascii_case("gmt")
        || time_zone == "Z"
    {
        return Ok(Some("UTC".to_string()));
    }

    Ok(Some(time_zone.to_string()))
}

/// Map one calendar trigger payload error into one runtime error.
fn notification_calendar_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "trigger",
        format!("calendar notification trigger is invalid: {error}"),
    ))
    .boxed()
}

/// Map one named timezone lookup error into one runtime error.
fn notification_named_time_zone_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "trigger",
        format!("calendar notification timezone is invalid: {error}"),
    ))
    .boxed()
}

/// Map one system timezone lookup error into one runtime error.
fn notification_system_time_zone_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("host system timezone lookup failed: {error}"),
    ))
    .boxed()
}
