use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundNetworkRequirementValue, BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
    BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
};

/// Prefix for durable desktop background scheduler artifacts.
pub(crate) const DESKTOP_BACKGROUND_SCHEDULER_PREFIX: &str = "destack-background";
/// Minimum desktop background interval accepted on Windows task scheduler.
pub(crate) const DESKTOP_BACKGROUND_WINDOWS_MINIMUM_INTERVAL_NS: u64 = 60_000_000_000;

/// Durable desktop background task record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DesktopBackgroundTaskRecord {
    /// Registered background task options.
    pub(crate) options: BackgroundTaskOptionsValue,
}

/// Validate one desktop background task payload against the current host model.
pub(crate) fn validate_desktop_background_options(
    platform: Platform,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    if options.identifier.trim().is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "identifier",
            "desktop background identifier must not be empty",
        ))
        .boxed());
    }

    // schedule kind
    match options.schedule.kind {
        BackgroundTaskScheduleKindValue::Once => {
            if options.schedule.repeat_interval_ns.is_some() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "schedule.repeatIntervalNs",
                    "once schedules must not define one repeat interval",
                ))
                .boxed());
            }
        }
        BackgroundTaskScheduleKindValue::Recurring => {
            let repeat_interval_ns = options.schedule.repeat_interval_ns.ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "schedule.repeatIntervalNs",
                    "recurring schedules require one repeat interval",
                ))
                .boxed()
            })?;

            if repeat_interval_ns == 0 {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "schedule.repeatIntervalNs",
                    "recurring schedules require one non-zero repeat interval",
                ))
                .boxed());
            }

            // task scheduler only accepts minute-granularity repetition
            if matches!(platform, Platform::Windows)
                && repeat_interval_ns < DESKTOP_BACKGROUND_WINDOWS_MINIMUM_INTERVAL_NS
            {
                return Err(not_supported("destack.os.background.register"));
            }
        }
    }

    // platform constraint support
    match platform {
        Platform::Linux | Platform::MacOS => {
            if options.network != BackgroundNetworkRequirementValue::None {
                return Err(desktop_background_not_supported_with_message(
                    "desktop background network constraints are not supported on this host",
                ));
            }

            if options.requires_charging {
                return Err(desktop_background_not_supported_with_message(
                    "desktop background charging constraints are not supported on this host",
                ));
            }

            if options.requires_idle {
                return Err(desktop_background_not_supported_with_message(
                    "desktop background idle constraints are not supported on this host",
                ));
            }

            // launchd can drive recurring timers directly, but not one delayed recurring stream
            if matches!(platform, Platform::MacOS)
                && options.schedule.kind == BackgroundTaskScheduleKindValue::Recurring
                && options.schedule.earliest_begin_unix_ns.is_some()
            {
                return Err(desktop_background_not_supported_with_message(
                    "macos background tasks do not support one delayed recurring start time",
                ));
            }
        }
        Platform::Windows => {
            if options.network == BackgroundNetworkRequirementValue::Unmetered {
                return Err(desktop_background_not_supported_with_message(
                    "windows background tasks do not support unmetered network constraints",
                ));
            }
        }
        _ => return Err(not_supported("destack.os.background.register")),
    }

    Ok(())
}

/// Return the first scheduled run time for one desktop background schedule.
pub(crate) fn background_first_run_unix_ns(
    schedule: &BackgroundTaskScheduleValue,
) -> RuntimeResult<u64> {
    let wall_clock_now_ns = background_wall_clock_now_ns()?;

    match schedule.kind {
        BackgroundTaskScheduleKindValue::Once => Ok(schedule
            .earliest_begin_unix_ns
            .unwrap_or(wall_clock_now_ns)
            .max(wall_clock_now_ns)),
        BackgroundTaskScheduleKindValue::Recurring => {
            schedule.repeat_interval_ns.ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "schedule.repeatIntervalNs",
                    "recurring schedules require one repeat interval",
                ))
                .boxed()
            })?;

            Ok(schedule
                .earliest_begin_unix_ns
                .unwrap_or(wall_clock_now_ns)
                .max(wall_clock_now_ns))
        }
    }
}

/// Return one descriptor view for one durable task options payload.
pub(crate) fn background_descriptor_from_options(
    options: BackgroundTaskOptionsValue,
) -> BackgroundTaskDescriptorValue {
    BackgroundTaskDescriptorValue {
        identifier: options.identifier,
        trigger: options.trigger,
        schedule: options.schedule,
        network: options.network,
        requires_charging: options.requires_charging,
        requires_idle: options.requires_idle,
        conflict_policy: options.conflict_policy,
    }
}

/// Return the effective repetition interval in whole seconds.
pub(crate) fn background_interval_seconds(options: &BackgroundTaskOptionsValue) -> u64 {
    let interval_seconds = options
        .schedule
        .repeat_interval_ns
        .unwrap_or(0)
        .saturating_add(999_999_999)
        / 1_000_000_000;

    interval_seconds.max(1)
}

/// Return the current wall-clock time in Unix nanoseconds.
fn background_wall_clock_now_ns() -> RuntimeResult<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::Generic),
                format!("desktop background clock moved before the Unix epoch: {error}"),
            ))
            .boxed()
        })?;

    Ok(now.as_nanos().min(u128::from(u64::MAX)) as u64)
}

/// Return one desktop background `notSupported` error with one custom message.
fn desktop_background_not_supported_with_message(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(message.into())).boxed()
}

/// Remove one persisted background file when it exists.
pub(crate) fn remove_background_file_if_exists(
    path: &Path,
    operation: &'static str,
) -> RuntimeResult<()> {
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

#[cfg(test)]
mod tests {
    use destack_artifact::Platform;

    use super::{background_first_run_unix_ns, validate_desktop_background_options};
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::os::abi_generated::{
        BackgroundConflictPolicyValue, BackgroundNetworkRequirementValue,
        BackgroundTaskOptionsValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
        BackgroundTriggerKindValue,
    };
    use crate::tests::platform::assert_runtime_error_code;

    /// Reject unsupported Linux desktop background network constraints.
    #[test]
    fn test_validate_desktop_background_options_rejects_linux_network_constraints() {
        let error = validate_desktop_background_options(
            Platform::Linux,
            &background_options(BackgroundNetworkRequirementValue::Connected, false, false),
        )
        .expect_err("linux desktop background network constraints should be rejected");

        assert_runtime_error_code(&error, PlatformErrorCode::NotSupported);
    }

    /// Reject unsupported macOS desktop background charging constraints.
    #[test]
    fn test_validate_desktop_background_options_rejects_macos_charging_constraints() {
        let error = validate_desktop_background_options(
            Platform::MacOS,
            &background_options(BackgroundNetworkRequirementValue::None, true, false),
        )
        .expect_err("macos desktop background charging constraints should be rejected");

        assert_runtime_error_code(&error, PlatformErrorCode::NotSupported);
    }

    /// Reject one delayed recurring macOS desktop background schedule.
    #[test]
    fn test_validate_desktop_background_options_rejects_macos_delayed_recurring_schedule() {
        let error = validate_desktop_background_options(
            Platform::MacOS,
            &BackgroundTaskOptionsValue {
                schedule: BackgroundTaskScheduleValue {
                    kind: BackgroundTaskScheduleKindValue::Recurring,
                    earliest_begin_unix_ns: Some(1_900_000_000_000_000_000),
                    repeat_interval_ns: Some(300_000_000_000),
                },
                ..background_options(BackgroundNetworkRequirementValue::None, false, false)
            },
        )
        .expect_err("macos delayed recurring desktop background schedules should be rejected");

        assert_runtime_error_code(&error, PlatformErrorCode::NotSupported);
    }

    /// Reject unsupported Windows desktop unmetered constraints.
    #[test]
    fn test_validate_desktop_background_options_rejects_windows_unmetered_constraints() {
        let error = validate_desktop_background_options(
            Platform::Windows,
            &background_options(BackgroundNetworkRequirementValue::Unmetered, false, false),
        )
        .expect_err("windows desktop background unmetered constraints should be rejected");

        assert_runtime_error_code(&error, PlatformErrorCode::NotSupported);
    }

    /// Schedule recurring desktop tasks from the earliest eligible instant.
    #[test]
    fn test_background_first_run_unix_ns_recurring_does_not_wait_one_full_interval() {
        let schedule = BackgroundTaskScheduleValue {
            kind: BackgroundTaskScheduleKindValue::Recurring,
            earliest_begin_unix_ns: None,
            repeat_interval_ns: Some(300_000_000_000),
        };
        let first_run_unix_ns =
            background_first_run_unix_ns(&schedule).expect("recurring first run should resolve");
        let now_unix_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after the unix epoch")
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;

        assert!(first_run_unix_ns >= now_unix_ns);
        assert!(first_run_unix_ns < now_unix_ns.saturating_add(300_000_000_000));
    }

    /// Build one desktop background options payload for validation tests.
    fn background_options(
        network: BackgroundNetworkRequirementValue,
        requires_charging: bool,
        requires_idle: bool,
    ) -> BackgroundTaskOptionsValue {
        BackgroundTaskOptionsValue {
            identifier: "sync".to_string(),
            trigger: BackgroundTriggerKindValue::AppRefresh,
            schedule: BackgroundTaskScheduleValue {
                kind: BackgroundTaskScheduleKindValue::Recurring,
                earliest_begin_unix_ns: None,
                repeat_interval_ns: Some(300_000_000_000),
            },
            network,
            requires_charging,
            requires_idle,
            conflict_policy: BackgroundConflictPolicyValue::Replace,
        }
    }
}
