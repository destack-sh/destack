use std::fs;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use tracing::warn;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostRequestContext;
use crate::host::unix::request::background::systemd::SystemdManager;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::NotificationPriority;
use crate::platform::os::abi_generated::{
    NotificationCalendarTriggerValue, NotificationImmediateTriggerValue, NotificationRequestValue,
    NotificationScheduledDescriptorValue, NotificationTimeIntervalTriggerValue,
    NotificationTriggerValue,
};
use crate::platform::os::notification::storage::{
    DesktopScheduledNotificationRecord, notification_scheduler_key,
    notification_systemd_service_path, notification_systemd_timer_path,
    notification_wrapper_script_path, read_notification_records, remove_notification_record,
    write_notification_record,
};
use crate::platform::os::notification::time::{
    calendar_date_trigger_unix_ns, notification_time_zone_suffix,
};
use crate::platform::os::notification::wrapper::{
    systemd_quote_argument, write_notification_wrapper_script,
};

/// Schedule one notification through one Linux systemd user timer.
pub(crate) fn schedule_notification(
    context: &HostRequestContext,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<()> {
    let wrapper_path = write_notification_wrapper_script(context, id)?;
    let service_path = notification_systemd_service_path(id)?;
    let timer_path = notification_systemd_timer_path(id)?;
    let systemd_directory = service_path.parent().ok_or_else(|| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            "destack.os.notification.schedule: invalid systemd user path",
        ))
        .boxed()
    })?;
    let key = notification_scheduler_key(id);
    let timer_spec = notification_timer_spec(request)?;
    let service_contents = render_systemd_service(&key, &wrapper_path);
    let timer_contents = render_systemd_timer(&key, &timer_spec);
    let record = DesktopScheduledNotificationRecord {
        id: id.to_string(),
        request: request.clone(),
        scheduled_unix_ns: Some(timer_spec.scheduled_unix_ns),
    };

    let registration_result = (|| {
        fs::create_dir_all(systemd_directory).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.notification.schedule: failed to create systemd user directory {}: {error}",
                    systemd_directory.display()
                ),
            ))
            .boxed()
        })?;

        fs::write(&service_path, &service_contents).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.notification.schedule: failed to write systemd service {}: {error}",
                    service_path.display()
                ),
            ))
            .boxed()
        })?;

        fs::write(&timer_path, &timer_contents).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.notification.schedule: failed to write systemd timer {}: {error}",
                    timer_path.display()
                ),
            ))
            .boxed()
        })?;

        write_notification_record(context, &record)?;

        let manager = SystemdManager::connect("destack.os.notification.schedule")?;
        let timer_unit_name = format!("{key}.timer");

        manager.reload("destack.os.notification.schedule")?;
        manager.enable_unit(&timer_unit_name, "destack.os.notification.schedule")?;
        manager.start_unit(&timer_unit_name, "destack.os.notification.schedule")?;

        Ok(())
    })();

    if let Err(error) = registration_result {
        if let Err(cleanup_error) =
            cancel_scheduled_notification(context, id, false, "destack.os.notification.schedule")
        {
            warn!(
                ?cleanup_error,
                notification_id = id,
                "failed to roll back Linux scheduled notification registration"
            );
        }

        return Err(error);
    }

    Ok(())
}

/// Return pending Linux scheduled notifications from persisted scheduler state.
pub(crate) fn list_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    let records = read_notification_records(context)?;
    let descriptors = records
        .into_iter()
        .map(|record| record.descriptor())
        .collect();

    Ok(descriptors)
}

/// Cancel one pending Linux scheduled notification.
pub(crate) fn cancel_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    cancel_scheduled_notification(context, id, true, "destack.os.notification.pendingCancel")
}

/// Reject unsupported bulk pending notification cancellation.
pub(super) fn cancel_all_pending_notifications() -> RuntimeResult<()> {
    unreachable!("bulk pending notification cancellation is handled in shared runtime logic")
}

/// Remove one scheduled Linux notification from systemd and persisted state.
pub(super) fn cancel_scheduled_notification(
    context: &HostRequestContext,
    id: &str,
    stop_loaded_service: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let key = notification_scheduler_key(id);
    let service_path = notification_systemd_service_path(id)?;
    let timer_path = notification_systemd_timer_path(id)?;
    let wrapper_path = notification_wrapper_script_path(context, id)?;
    let timer_unit_name = format!("{key}.timer");
    let service_unit_name = format!("{key}.service");
    let manager = SystemdManager::connect(operation)?;

    // stop the timer and optional active service before removing unit files
    manager.stop_unit_if_present(&timer_unit_name, operation)?;

    if stop_loaded_service {
        manager.stop_unit_if_present(&service_unit_name, operation)?;
    }

    manager.disable_unit_if_present(&timer_unit_name, operation)?;

    // persisted artifacts
    remove_notification_record(context, id)?;
    remove_file_if_exists(&service_path, operation)?;
    remove_file_if_exists(&timer_path, operation)?;
    remove_file_if_exists(&wrapper_path, operation)?;

    // flush removed definitions from systemd
    manager.reload(operation)?;

    Ok(())
}

/// Return whether one notification trigger repeats.
pub(super) fn notification_trigger_repeats(trigger: &NotificationTriggerValue) -> bool {
    match trigger {
        NotificationTriggerValue::NotificationImmediateTrigger(_) => false,
        NotificationTriggerValue::NotificationTimeIntervalTrigger(_) => false,
        NotificationTriggerValue::NotificationCalendarDateTrigger(value) => value.calendar.repeats,
    }
}

/// Remove one file when it exists.
fn remove_file_if_exists(path: &std::path::Path, operation: &'static str) -> RuntimeResult<()> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("{operation}: failed to remove {}: {error}", path.display()),
        ))
        .boxed()
    })?;

    Ok(())
}

/// Timer specification for one Linux scheduled notification.
struct NotificationTimerSpec {
    /// Earliest scheduled delivery timestamp in UTC nanoseconds.
    scheduled_unix_ns: u64,
    /// Timer body lines that follow the [Timer] header.
    body: String,
}

/// Build one systemd timer specification from one notification trigger.
fn notification_timer_spec(
    request: &NotificationRequestValue,
) -> RuntimeResult<NotificationTimerSpec> {
    match &request.trigger {
        NotificationTriggerValue::NotificationImmediateTrigger(_) => {
            let scheduled_unix_ns = wall_clock_now_ns()?;
            let calendar = systemd_unix_timestamp_expression(scheduled_unix_ns)?;

            Ok(NotificationTimerSpec {
                scheduled_unix_ns,
                body: format!("OnCalendar={calendar}\nPersistent=true\n"),
            })
        }
        NotificationTriggerValue::NotificationTimeIntervalTrigger(value) => {
            let scheduled_unix_ns = wall_clock_now_ns()?.saturating_add(value.interval_ns);
            let calendar = systemd_unix_timestamp_expression(scheduled_unix_ns)?;

            Ok(NotificationTimerSpec {
                scheduled_unix_ns,
                body: format!("OnCalendar={calendar}\nPersistent=true\n"),
            })
        }
        NotificationTriggerValue::NotificationCalendarDateTrigger(value) => {
            let scheduled_unix_ns = calendar_date_trigger_unix_ns(value)?;
            let calendar = systemd_calendar_expression(&value.calendar)?;

            Ok(NotificationTimerSpec {
                scheduled_unix_ns,
                body: format!("OnCalendar={calendar}\nPersistent=true\n"),
            })
        }
    }
}

/// Return the systemd calendar expression for one notification calendar trigger.
fn systemd_calendar_expression(value: &NotificationCalendarTriggerValue) -> RuntimeResult<String> {
    let year = if value.repeats {
        "*".to_string()
    } else {
        value.year.to_string()
    };
    let month = format!("{:02}", value.month);
    let day = format!("{:02}", value.day);
    let hour = format!("{:02}", value.hour);
    let minute = format!("{:02}", value.minute);
    let second = format!("{:02}", value.second);
    let suffix = notification_time_zone_suffix(&value.time_zone)?;

    if let Some(suffix) = suffix {
        return Ok(format!(
            "{year}-{month}-{day} {hour}:{minute}:{second} {suffix}"
        ));
    }

    Ok(format!("{year}-{month}-{day} {hour}:{minute}:{second}"))
}

/// Return one UTC systemd calendar expression for one Unix timestamp.
fn systemd_unix_timestamp_expression(unix_ns: u64) -> RuntimeResult<String> {
    let timestamp = Timestamp::from_nanosecond(i128::from(unix_ns)).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "trigger",
            format!("scheduled notification timestamp is invalid: {error}"),
        ))
        .boxed()
    })?;
    let zoned = timestamp.to_zoned(TimeZone::UTC);

    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        zoned.year(),
        zoned.month(),
        zoned.day(),
        zoned.hour(),
        zoned.minute(),
        zoned.second(),
    ))
}

/// Return the current wall-clock time in Unix nanoseconds.
fn wall_clock_now_ns() -> RuntimeResult<u64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::Generic),
                format!("notification runtime clock moved before the Unix epoch: {error}"),
            ))
            .boxed()
        })?;

    Ok(now.as_nanos().min(u128::from(u64::MAX)) as u64)
}

/// Render one systemd service unit for one notification wrapper script.
fn render_systemd_service(key: &str, script_path: &std::path::Path) -> String {
    let script_path = systemd_quote_argument(&script_path.to_string_lossy());

    format!(
        "[Unit]\nDescription=Destack scheduled notification {key}\n\n[Service]\nType=exec\nExecStart={script_path}\n"
    )
}

/// Render one systemd timer unit for one scheduled notification.
fn render_systemd_timer(key: &str, timer_spec: &NotificationTimerSpec) -> String {
    format!(
        "[Unit]\nDescription=Destack scheduled notification timer {key}\n\n[Timer]\n{}Unit={key}.service\n\n[Install]\nWantedBy=timers.target\n",
        timer_spec.body
    )
}

#[cfg(test)]
mod tests {
    use super::{notification_timer_spec, systemd_calendar_expression};
    use crate::platform::os::NotificationPriority;
    use crate::platform::os::abi_generated::{
        NotificationCalendarDateTriggerValue, NotificationCalendarTriggerValue,
        NotificationImmediateTriggerValue, NotificationRequestValue,
        NotificationTimeIntervalTriggerValue, NotificationTriggerValue,
    };

    /// Render one repeating calendar timer with a wildcard year.
    #[test]
    fn test_systemd_calendar_expression_repeats_annually() {
        let expression = systemd_calendar_expression(&NotificationCalendarTriggerValue {
            year: 2026,
            month: 5,
            day: 7,
            hour: 12,
            minute: 34,
            second: 56,
            time_zone: "UTC".to_string(),
            repeats: true,
        })
        .expect("calendar expression should render");

        assert_eq!(expression, "*-05-07 12:34:56 UTC");
    }

    /// Render one immediate timer with a microsecond active delay.
    #[test]
    fn test_notification_timer_spec_uses_absolute_immediate_deadline() {
        let timer_spec = notification_timer_spec(&NotificationRequestValue {
            title: "title".to_string(),
            subtitle: None,
            body: "body".to_string(),
            tag: "tag".to_string(),
            channel_id: None,
            priority: NotificationPriority::Normal,
            badge_count: None,
            sound: None,
            category_id: None,
            thread_id: None,
            trigger: NotificationTriggerValue::NotificationImmediateTrigger(
                NotificationImmediateTriggerValue {
                    kind: "immediate".to_string(),
                },
            ),
            action_id: None,
        })
        .expect("timer spec should render");

        assert!(timer_spec.body.starts_with("OnCalendar="));
        assert!(timer_spec.body.ends_with(" UTC\nPersistent=true\n"));
    }

    /// Preserve one named timezone in the rendered systemd calendar timer.
    #[test]
    fn test_notification_timer_spec_preserves_named_calendar_timezones() {
        let timer_spec = notification_timer_spec(&NotificationRequestValue {
            title: "title".to_string(),
            subtitle: None,
            body: "body".to_string(),
            tag: "tag".to_string(),
            channel_id: None,
            priority: NotificationPriority::Normal,
            badge_count: None,
            sound: None,
            category_id: None,
            thread_id: None,
            trigger: NotificationTriggerValue::NotificationCalendarDateTrigger(
                NotificationCalendarDateTriggerValue {
                    kind: "calendarDate".to_string(),
                    calendar: NotificationCalendarTriggerValue {
                        year: 2026,
                        month: 5,
                        day: 7,
                        hour: 12,
                        minute: 34,
                        second: 56,
                        time_zone: "Europe/Zurich".to_string(),
                        repeats: false,
                    },
                },
            ),
            action_id: None,
        })
        .expect("named calendar timezones should render");

        assert_eq!(
            timer_spec.body,
            "OnCalendar=2026-05-07 12:34:56 Europe/Zurich\nPersistent=true\n"
        );
    }

    /// Round one time-interval timer up to whole seconds for systemd.
    #[test]
    fn test_notification_timer_spec_uses_absolute_time_interval_deadline() {
        let timer_spec = notification_timer_spec(&NotificationRequestValue {
            title: "title".to_string(),
            subtitle: None,
            body: "body".to_string(),
            tag: "tag".to_string(),
            channel_id: None,
            priority: NotificationPriority::Normal,
            badge_count: None,
            sound: None,
            category_id: None,
            thread_id: None,
            trigger: NotificationTriggerValue::NotificationTimeIntervalTrigger(
                NotificationTimeIntervalTriggerValue {
                    kind: "timeInterval".to_string(),
                    interval_ns: 1_500_000_000,
                },
            ),
            action_id: None,
        })
        .expect("timer spec should render");

        assert!(timer_spec.body.starts_with("OnCalendar="));
        assert!(timer_spec.body.ends_with(" UTC\nPersistent=true\n"));
    }
}
