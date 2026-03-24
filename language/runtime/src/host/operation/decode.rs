use crate::diagnostic::RuntimeResult;
use crate::host::core::request::{
    HostRequestOutcome, HostRequestResult, unexpected_request_result,
};
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskDescriptorValue, CalendarDescriptorValue,
    CalendarEventValue, ContactPageValue, ContactValue, LocationSampleValue,
    MediaAssetDescriptorValue, MediaPageValue, NotificationCategoryValue,
    NotificationScheduledDescriptorValue,
};

/// Decode one unit request result.
pub(crate) fn none(outcome: HostRequestOutcome, operation: &'static str) -> RuntimeResult<()> {
    match outcome.result {
        HostRequestResult::None => Ok(()),
        _ => Err(unexpected_request_result(operation, "empty result")),
    }
}

/// Decode one boolean request result.
pub(crate) fn bool_value(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<bool> {
    match outcome.result {
        HostRequestResult::Bool(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "bool")),
    }
}

/// Decode one string request result.
pub(crate) fn string(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<String> {
    match outcome.result {
        HostRequestResult::String(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "string")),
    }
}

/// Decode one unsigned 32-bit request result.
pub(crate) fn u32_value(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<u32> {
    match outcome.result {
        HostRequestResult::U32(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "u32")),
    }
}

/// Decode one background scheduler status request result.
pub(crate) fn background_status(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<BackgroundStatusValue> {
    match outcome.result {
        HostRequestResult::BackgroundStatus(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "background status")),
    }
}

/// Decode one background task descriptor list request result.
pub(crate) fn background_task_descriptors(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    match outcome.result {
        HostRequestResult::BackgroundTaskDescriptors(value) => Ok(value),
        _ => Err(unexpected_request_result(
            operation,
            "background task descriptors",
        )),
    }
}

/// Decode one calendar descriptor list request result.
pub(crate) fn calendar_descriptors(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    match outcome.result {
        HostRequestResult::CalendarDescriptors(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "calendar descriptors")),
    }
}

/// Decode one calendar event list request result.
pub(crate) fn calendar_events(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    match outcome.result {
        HostRequestResult::CalendarEvents(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "calendar events")),
    }
}

/// Decode one calendar event request result.
pub(crate) fn calendar_event(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<CalendarEventValue> {
    match outcome.result {
        HostRequestResult::CalendarEvent(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "calendar event")),
    }
}

/// Decode one contact page request result.
pub(crate) fn contact_page(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<ContactPageValue> {
    match outcome.result {
        HostRequestResult::ContactPage(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "contact page")),
    }
}

/// Decode one contact request result.
pub(crate) fn contact(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<ContactValue> {
    match outcome.result {
        HostRequestResult::Contact(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "contact")),
    }
}

/// Decode one location sample request result.
pub(crate) fn location_sample(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<LocationSampleValue> {
    match outcome.result {
        HostRequestResult::LocationSample(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "location sample")),
    }
}

/// Decode one media asset descriptor request result.
pub(crate) fn media_asset_descriptor(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    match outcome.result {
        HostRequestResult::MediaAssetDescriptor(value) => Ok(value),
        _ => Err(unexpected_request_result(
            operation,
            "media asset descriptor",
        )),
    }
}

/// Decode one media page request result.
pub(crate) fn media_page(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<MediaPageValue> {
    match outcome.result {
        HostRequestResult::MediaPage(value) => Ok(value),
        _ => Err(unexpected_request_result(operation, "media page")),
    }
}

/// Decode one notification permission request result.
pub(crate) fn notification_permission_state(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<NotificationPermissionState> {
    match outcome.result {
        HostRequestResult::NotificationPermissionState(value) => Ok(value),
        _ => Err(unexpected_request_result(
            operation,
            "notification permission state",
        )),
    }
}

/// Decode one notification identifier request result.
pub(crate) fn notification_id(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<String> {
    match outcome.result {
        HostRequestResult::NotificationId(value) => Ok(value),
        _ => Err(unexpected_request_result(
            operation,
            "notification identifier",
        )),
    }
}

/// Decode one notification category list request result.
pub(crate) fn notification_categories(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<Vec<NotificationCategoryValue>> {
    match outcome.result {
        HostRequestResult::NotificationCategories(value) => Ok(value),
        _ => Err(unexpected_request_result(
            operation,
            "notification categories",
        )),
    }
}

/// Decode one scheduled notification descriptor list request result.
pub(crate) fn notification_scheduled_descriptors(
    outcome: HostRequestOutcome,
    operation: &'static str,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    match outcome.result {
        HostRequestResult::NotificationScheduledDescriptors(value) => Ok(value),
        _ => Err(unexpected_request_result(
            operation,
            "notification scheduled descriptors",
        )),
    }
}
