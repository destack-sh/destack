use objc2::rc::Retained;
use objc2_event_kit::EKEventStore;
use objc2_foundation::{NSDate, NSString, NSTimeZone, NSURL};

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::CalendarEventValue;

/// The calendar event-list operation name.
pub(super) const CALENDAR_EVENT_LIST_OPERATION: &str = "destack.os.calendar.eventList";

/// The calendar event-read operation name.
pub(super) const CALENDAR_EVENT_READ_OPERATION: &str = "destack.os.calendar.eventRead";

/// The calendar event-create operation name.
pub(super) const CALENDAR_EVENT_CREATE_OPERATION: &str = "destack.os.calendar.eventCreate";

/// The calendar event-update operation name.
pub(super) const CALENDAR_EVENT_UPDATE_OPERATION: &str = "destack.os.calendar.eventUpdate";

/// The calendar event-delete operation name.
pub(super) const CALENDAR_EVENT_DELETE_OPERATION: &str = "destack.os.calendar.eventDelete";

/// Create one fresh EventKit store.
pub(super) fn new_event_store() -> Retained<EKEventStore> {
    unsafe { EKEventStore::new() }
}

/// Build one Foundation URL from one runtime string.
pub(super) fn ns_url_from_string(
    value: &str,
    operation: &'static str,
) -> crate::diagnostic::RuntimeResult<Retained<NSURL>> {
    let value = NSString::from_str(value);
    let url = NSURL::URLWithString(&value);

    url.ok_or_else(|| {
        calendar_invalid_argument(
            operation,
            "calendar event url must be one valid absolute URL",
        )
    })
}

/// Build one Foundation timezone from one runtime identifier.
pub(super) fn ns_time_zone_from_name(
    name: &str,
    operation: &'static str,
) -> crate::diagnostic::RuntimeResult<Retained<NSTimeZone>> {
    let name = NSString::from_str(name);
    let time_zone = NSTimeZone::timeZoneWithName(&name);

    time_zone.ok_or_else(|| {
        calendar_invalid_argument(
            operation,
            "calendar event time_zone must be one valid timezone identifier",
        )
    })
}

/// Build one Foundation date from one unix timestamp in nanoseconds.
pub(super) fn ns_date_from_unix_ns(unix_ns: u64) -> Retained<NSDate> {
    NSDate::dateWithTimeIntervalSince1970(unix_ns as f64 / 1_000_000_000.0)
}

/// Convert one Foundation date into unix timestamp nanoseconds.
pub(super) fn unix_ns_from_date(date: &NSDate) -> u64 {
    (date.timeIntervalSince1970() * 1_000_000_000.0)
        .round()
        .max(0.0) as u64
}

/// Clamp one normalized color channel into one byte.
pub(super) fn float_channel(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Build one loud calendar backend error.
pub(super) fn calendar_error(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("{operation}: {}", message.into()),
    ))
    .boxed()
}

/// Build one loud calendar invalid-argument error.
pub(super) fn calendar_invalid_argument(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "value",
        format!("{operation}: {}", message.into()),
    ))
    .boxed()
}

/// Collapse expanded recurring instances into one representative row per series.
pub(super) fn collapse_recurrence_instances(events: &mut Vec<CalendarEventValue>) {
    let mut seen_series = rustc_hash::FxHashSet::default();

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
