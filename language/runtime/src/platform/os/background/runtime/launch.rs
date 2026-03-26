use std::ffi::OsString;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::monotonic_now_ns;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::BackgroundTriggerKindValue;

use super::state::{
    DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV, DesktopBackgroundLaunchMarker,
    desktop_background_runtime_service,
};

/// Environment marker carrying one desktop background execution identifier.
const DESKTOP_BACKGROUND_EXECUTION_ID_ENV: &str = "DESTACK_BACKGROUND_EXECUTION_ID";

/// Environment marker carrying one desktop background execution deadline.
pub(crate) const DESKTOP_BACKGROUND_DEADLINE_UNIX_NS_ENV: &str =
    "DESTACK_BACKGROUND_DEADLINE_UNIX_NS";

/// Default runtime deadline for one app-refresh execution.
const DESKTOP_BACKGROUND_APP_REFRESH_DEADLINE_NS: u64 = 30_000_000_000;

/// Default runtime deadline for one processing execution.
const DESKTOP_BACKGROUND_PROCESSING_DEADLINE_NS: u64 = 3_600_000_000_000;

/// Default runtime deadline for one desktop test-triggered execution.
const DESKTOP_BACKGROUND_TRIGGER_DEADLINE_NS: u64 = 30_000_000_000;

/// Return the default runtime deadline for one desktop background trigger class.
pub(crate) fn desktop_background_execution_deadline_ns(trigger: BackgroundTriggerKindValue) -> u64 {
    match trigger {
        BackgroundTriggerKindValue::AppRefresh => DESKTOP_BACKGROUND_APP_REFRESH_DEADLINE_NS,
        BackgroundTriggerKindValue::Processing => DESKTOP_BACKGROUND_PROCESSING_DEADLINE_NS,
    }
}

/// Read one launch marker state from the current process environment.
pub(crate) fn desktop_background_registry_launch_marker_state()
-> (Option<DesktopBackgroundLaunchMarker>, Option<String>) {
    match desktop_background_launch_marker_from_environment(|name| std::env::var_os(name)) {
        Ok(launch_marker) => (launch_marker, None),
        Err(error) => (None, Some(error.message())),
    }
}

/// Return one process launch marker when the current process was started for background work.
fn desktop_background_launch_marker_from_environment(
    read_variable: impl Fn(&str) -> Option<OsString>,
) -> RuntimeResult<Option<DesktopBackgroundLaunchMarker>> {
    let Some(identifier) = read_variable(DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV) else {
        return Ok(None);
    };

    let identifier = identifier.into_string().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "identifier",
            "desktop background identifier must be valid UTF-8",
        ))
        .boxed()
    })?;
    let execution_id = match read_variable(DESKTOP_BACKGROUND_EXECUTION_ID_ENV) {
        Some(value) => value.into_string().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "executionId",
                "desktop background execution id must be valid UTF-8",
            ))
            .boxed()
        })?,
        None => format!("desktop-execution-{}", monotonic_now_ns()),
    };
    let deadline_unix_ns = match read_variable(DESKTOP_BACKGROUND_DEADLINE_UNIX_NS_ENV) {
        Some(value) => value
            .into_string()
            .map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "deadlineUnixNs",
                    "desktop background deadline must be valid UTF-8",
                ))
                .boxed()
            })?
            .parse::<u64>()
            .map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "deadlineUnixNs",
                    "desktop background deadline must be one uint64 value",
                ))
                .boxed()
            })?,
        None => wall_clock_now_ns()?.saturating_add(DESKTOP_BACKGROUND_APP_REFRESH_DEADLINE_NS),
    };

    Ok(Some(DesktopBackgroundLaunchMarker {
        identifier,
        execution_id,
        deadline_unix_ns,
    }))
}

/// Queue one synthetic background launch marker for desktop test mode.
pub(crate) fn enqueue_test_background_launch(identifier: &str) -> RuntimeResult<()> {
    let execution_id = format!("desktop-trigger-{}", monotonic_now_ns());
    let deadline_unix_ns =
        wall_clock_now_ns()?.saturating_add(DESKTOP_BACKGROUND_TRIGGER_DEADLINE_NS);
    let service = desktop_background_runtime_service();
    let mut registry = service.registry.lock();

    registry.launch_marker = Some(DesktopBackgroundLaunchMarker {
        identifier: identifier.to_string(),
        execution_id,
        deadline_unix_ns,
    });
    registry.launch_session_id = None;

    Ok(())
}

/// Return the current wall-clock time in Unix nanoseconds.
pub(crate) fn wall_clock_now_ns() -> RuntimeResult<u64> {
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use crate::platform::os::abi_generated::BackgroundTriggerKindValue;

    use super::{
        DESKTOP_BACKGROUND_DEADLINE_UNIX_NS_ENV, DESKTOP_BACKGROUND_EXECUTION_ID_ENV,
        DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV, DesktopBackgroundLaunchMarker,
        desktop_background_execution_deadline_ns,
        desktop_background_launch_marker_from_environment,
    };

    /// Parse one valid desktop background launch marker from environment values.
    #[test]
    fn test_desktop_background_launch_marker_from_environment() {
        let marker = desktop_background_launch_marker_from_environment(|name| match name {
            DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV => Some(OsString::from("sync")),
            DESKTOP_BACKGROUND_EXECUTION_ID_ENV => Some(OsString::from("execution-1")),
            DESKTOP_BACKGROUND_DEADLINE_UNIX_NS_ENV => Some(OsString::from("42")),
            _ => None,
        })
        .expect("marker should parse");

        assert_eq!(
            marker,
            Some(DesktopBackgroundLaunchMarker {
                identifier: "sync".to_string(),
                execution_id: "execution-1".to_string(),
                deadline_unix_ns: 42,
            })
        );
    }

    /// Ignore background launch parsing when the process does not carry a task marker.
    #[test]
    fn test_desktop_background_launch_marker_requires_identifier() {
        let marker = desktop_background_launch_marker_from_environment(|_| None)
            .expect("missing marker should be ignored");

        assert_eq!(marker, None);
    }

    /// Synthesize one execution id when the scheduler only passes the task identifier.
    #[test]
    fn test_desktop_background_launch_marker_synthesizes_execution_id() {
        let marker = desktop_background_launch_marker_from_environment(|name| match name {
            DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV => Some(OsString::from("sync")),
            _ => None,
        })
        .expect("marker should parse")
        .expect("marker should exist");

        assert_eq!(marker.identifier, "sync");
        assert!(marker.execution_id.starts_with("desktop-execution-"));
        assert!(marker.deadline_unix_ns > 0);
    }

    /// Return distinct desktop execution deadlines for each trigger class.
    #[test]
    fn test_desktop_background_execution_deadline_ns_differs_by_trigger() {
        let app_refresh_deadline_ns =
            desktop_background_execution_deadline_ns(BackgroundTriggerKindValue::AppRefresh);
        let processing_deadline_ns =
            desktop_background_execution_deadline_ns(BackgroundTriggerKindValue::Processing);

        assert!(processing_deadline_ns > app_refresh_deadline_ns);
    }
}
