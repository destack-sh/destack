use std::fs;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::RequestContext;
use crate::host::os::unix::request::background::systemd::SystemdManager;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskOptionsValue, BackgroundTaskScheduleKindValue,
};

use crate::platform::os::background::runtime::{
    desktop_background_test_mode_enabled, enqueue_test_background_launch,
};
use crate::platform::os::background::storage::{
    background_first_run_unix_ns, background_interval_seconds, background_scheduler_key,
    background_systemd_service_path, background_systemd_timer_path, background_wrapper_script_path,
    remove_background_file_if_exists,
};
use crate::platform::os::background::wrapper::{
    systemd_quote_argument, write_background_wrapper_script,
};

/// Return whether the active host exposes one Linux background scheduler.
pub(crate) fn background_status() -> RuntimeResult<BackgroundStatusValue> {
    if desktop_background_test_mode_enabled() {
        return Ok(BackgroundStatusValue::Available);
    }

    if SystemdManager::connect("destack.os.background.status").is_ok() {
        return Ok(BackgroundStatusValue::Available);
    }

    Ok(BackgroundStatusValue::Unavailable)
}

/// Register one systemd-backed desktop background task.
pub(crate) fn register_background_task(
    context: &RequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    let wrapper_path =
        write_background_wrapper_script(context, &options.identifier, options.trigger)?;
    let service_path = background_systemd_service_path(&options.identifier)?;
    let timer_path = background_systemd_timer_path(&options.identifier)?;
    let systemd_directory = service_path.parent().ok_or_else(|| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            "destack.os.background.register: invalid systemd user path",
        ))
        .boxed()
    })?;
    let key = background_scheduler_key(&options.identifier);
    let first_run_unix_ns = background_first_run_unix_ns(&options.schedule)?;
    let service_contents = render_systemd_service(&key, &wrapper_path);
    let timer_contents = render_systemd_timer(&key, options, first_run_unix_ns)?;

    let registration_result = (|| {
        // scheduler artifacts
        fs::create_dir_all(systemd_directory).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.register: failed to create systemd user directory {}: {error}",
                    systemd_directory.display()
                ),
            ))
            .boxed()
        })?;

        fs::write(&service_path, service_contents).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.register: failed to write systemd service {}: {error}",
                    service_path.display()
                ),
            ))
            .boxed()
        })?;

        fs::write(&timer_path, timer_contents).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.register: failed to write systemd timer {}: {error}",
                    timer_path.display()
                ),
            ))
            .boxed()
        })?;

        if desktop_background_test_mode_enabled() {
            return Ok(());
        }

        // systemd manager
        let manager = SystemdManager::connect("destack.os.background.register")?;
        let timer_unit_name = format!("{key}.timer");

        manager.reload("destack.os.background.register")?;
        manager.enable_unit(&timer_unit_name, "destack.os.background.register")?;
        manager.start_unit(&timer_unit_name, "destack.os.background.register")?;

        Ok(())
    })();

    // rollback artifacts when scheduler registration fails partway through
    if let Err(error) = registration_result {
        if let Err(cleanup_error) = unregister_background_task(context, &options.identifier) {
            tracing::warn!(
                ?cleanup_error,
                identifier = options.identifier,
                "failed to roll back Linux background task registration"
            );
        }

        return Err(error);
    }

    Ok(())
}

/// Remove one systemd-backed desktop background task.
pub(crate) fn unregister_background_task(
    context: &RequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    let key = background_scheduler_key(identifier);
    let service_path = background_systemd_service_path(identifier)?;
    let timer_path = background_systemd_timer_path(identifier)?;
    let wrapper_path = background_wrapper_script_path(context, identifier)?;

    if !desktop_background_test_mode_enabled() {
        let timer_unit_name = format!("{key}.timer");
        let service_unit_name = format!("{key}.service");
        let manager = SystemdManager::connect("destack.os.background.unregister")?;

        // stop the timer and any active service before removing unit files
        manager.stop_unit_if_present(&timer_unit_name, "destack.os.background.unregister")?;
        manager.stop_unit_if_present(&service_unit_name, "destack.os.background.unregister")?;
        manager.disable_unit_if_present(&timer_unit_name, "destack.os.background.unregister")?;
    }

    // persisted artifacts
    remove_background_file_if_exists(&service_path, "destack.os.background.unregister")?;
    remove_background_file_if_exists(&timer_path, "destack.os.background.unregister")?;
    remove_background_file_if_exists(&wrapper_path, "destack.os.background.unregister")?;

    if !desktop_background_test_mode_enabled() {
        let manager = SystemdManager::connect("destack.os.background.unregister")?;

        // flush the removed unit definitions from systemd
        manager.reload("destack.os.background.unregister")?;
    }

    Ok(())
}

/// Trigger one systemd-backed background task immediately.
pub(crate) fn trigger_background_task(identifier: &str) -> RuntimeResult<bool> {
    if desktop_background_test_mode_enabled() {
        enqueue_test_background_launch(identifier)?;
        return Ok(true);
    }

    let key = background_scheduler_key(identifier);
    let service_unit_name = format!("{key}.service");
    let manager = SystemdManager::connect("destack.os.background.triggerTest")?;

    manager.start_unit(&service_unit_name, "destack.os.background.triggerTest")?;

    Ok(true)
}
/// Render one systemd service unit for one wrapper script.
fn render_systemd_service(key: &str, script_path: &std::path::Path) -> String {
    let script_path = systemd_quote_argument(&script_path.to_string_lossy());

    format!(
        "[Unit]\nDescription=Destack background task {key}\n\n[Service]\nType=exec\nExecStart={script_path}\n"
    )
}

/// Render one systemd timer unit for one background schedule.
fn render_systemd_timer(
    key: &str,
    options: &BackgroundTaskOptionsValue,
    first_run_unix_ns: u64,
) -> RuntimeResult<String> {
    let first_run_expression = systemd_unix_timestamp_expression(first_run_unix_ns)?;
    let timer_body = match options.schedule.kind {
        BackgroundTaskScheduleKindValue::Once => {
            format!("OnCalendar={first_run_expression}\nPersistent=true\n")
        }
        BackgroundTaskScheduleKindValue::Recurring => {
            let interval_seconds = background_interval_seconds(options);

            format!(
                "OnCalendar={first_run_expression}\nOnUnitActiveSec={interval_seconds}s\nPersistent=true\n"
            )
        }
    };

    Ok(format!(
        "[Unit]\nDescription=Destack background task timer {key}\n\n[Timer]\n{timer_body}Unit={key}.service\n\n[Install]\nWantedBy=timers.target\n"
    ))
}

/// Return one UTC systemd calendar expression for one Unix timestamp.
fn systemd_unix_timestamp_expression(unix_ns: u64) -> RuntimeResult<String> {
    let timestamp = Timestamp::from_nanosecond(i128::from(unix_ns)).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "schedule.earliestBeginUnixNs",
            format!("desktop background timestamp is invalid: {error}"),
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

#[cfg(test)]
mod tests {
    use crate::platform::os::abi_generated::{
        BackgroundConflictPolicyValue, BackgroundNetworkRequirementValue,
        BackgroundTaskOptionsValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
        BackgroundTriggerKindValue,
    };

    use super::render_systemd_timer;

    /// Render one recurring timer with one absolute first run and one repeat interval.
    #[test]
    fn test_render_systemd_timer_renders_recurring_calendar_and_repeat_lines() {
        let timer = render_systemd_timer(
            "destack-background-1234",
            &BackgroundTaskOptionsValue {
                identifier: "sync".to_string(),
                trigger: BackgroundTriggerKindValue::AppRefresh,
                schedule: BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Recurring,
                    earliest_begin_unix_ns: None,
                    repeat_interval_ns: Some(300_000_000_000),
                },
                network: BackgroundNetworkRequirementValue::None,
                requires_charging: false,
                requires_idle: false,
                conflict_policy: BackgroundConflictPolicyValue::Replace,
            },
            1_700_000_000_000_000_000,
        )
        .expect("recurring timer should render");

        assert!(timer.contains("OnCalendar="));
        assert!(timer.contains("OnUnitActiveSec=300s"));
        assert!(timer.contains("Persistent=true"));
    }

    /// Render one one-shot timer with one absolute calendar firing.
    #[test]
    fn test_render_systemd_timer_renders_one_shot_calendar_line() {
        let timer = render_systemd_timer(
            "destack-background-1234",
            &BackgroundTaskOptionsValue {
                identifier: "sync".to_string(),
                trigger: BackgroundTriggerKindValue::AppRefresh,
                schedule: BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Once,
                    earliest_begin_unix_ns: Some(1_700_000_000_000_000_000),
                    repeat_interval_ns: None,
                },
                network: BackgroundNetworkRequirementValue::None,
                requires_charging: false,
                requires_idle: false,
                conflict_policy: BackgroundConflictPolicyValue::Replace,
            },
            1_700_000_000_000_000_000,
        )
        .expect("one-shot timer should render");

        assert!(timer.contains("OnCalendar="));
        assert!(!timer.contains("OnUnitActiveSec="));
        assert!(timer.contains("Persistent=true"));
    }
}
