use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::os::abi_generated::NotificationTriggerValue;
use crate::platform::os::notification::time::calendar_date_trigger_unix_ns;

use super::core::macos_notification_time_error;

/// Build one native macOS trigger for one notification trigger payload.
pub(super) fn build_notification_trigger(
    trigger: &NotificationTriggerValue,
) -> RuntimeResult<objc2::rc::Retained<objc2_user_notifications::UNNotificationTrigger>> {
    use objc2_foundation::{NSDateComponents, NSString, NSTimeZone};
    use objc2_user_notifications::{
        UNCalendarNotificationTrigger, UNTimeIntervalNotificationTrigger,
    };

    match trigger {
        NotificationTriggerValue::NotificationImmediateTrigger(_) => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "trigger",
                "immediate notifications must use post instead of schedule",
            ))
            .boxed())
        }
        NotificationTriggerValue::NotificationTimeIntervalTrigger(value) => {
            let seconds = (value.interval_ns as f64) / 1_000_000_000.0;
            let trigger =
                UNTimeIntervalNotificationTrigger::triggerWithTimeInterval_repeats(seconds, false);

            Ok(trigger.into_super())
        }
        NotificationTriggerValue::NotificationCalendarDateTrigger(value) => {
            let components = NSDateComponents::new();
            components.setYear(value.calendar.year as isize);
            components.setMonth(value.calendar.month as isize);
            components.setDay(value.calendar.day as isize);
            components.setHour(value.calendar.hour as isize);
            components.setMinute(value.calendar.minute as isize);
            components.setSecond(value.calendar.second as isize);

            if !value.calendar.time_zone.is_empty() {
                let time_zone = NSString::from_str(&value.calendar.time_zone);
                let Some(time_zone) = NSTimeZone::timeZoneWithName(&time_zone) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "trigger",
                        "calendar notification timezone must be one valid host timezone",
                    ))
                    .boxed());
                };

                components.setTimeZone(Some(&time_zone));
            }

            let trigger = UNCalendarNotificationTrigger::triggerWithDateMatchingComponents_repeats(
                &components,
                value.calendar.repeats,
            );

            Ok(trigger.into_super())
        }
    }
}

/// Return the wall-clock delivery timestamp for one notification trigger.
pub(super) fn trigger_delivery_unix_ns(trigger: &NotificationTriggerValue) -> RuntimeResult<u64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(macos_notification_time_error)?;
    let now_unix_ns = now.as_nanos().min(u128::from(u64::MAX)) as u64;

    match trigger {
        NotificationTriggerValue::NotificationImmediateTrigger(_) => Ok(now_unix_ns),
        NotificationTriggerValue::NotificationTimeIntervalTrigger(value) => {
            Ok(now_unix_ns.saturating_add(value.interval_ns))
        }
        NotificationTriggerValue::NotificationCalendarDateTrigger(value) => {
            calendar_date_trigger_unix_ns(value)
        }
    }
}
