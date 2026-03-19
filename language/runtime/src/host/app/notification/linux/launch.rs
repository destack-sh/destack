use std::sync::OnceLock;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::app::notification::{runtime, unix};
use crate::host::core::{HostRequestContext, HostRuntimeId};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use super::schedule::{cancel_scheduled_notification, notification_trigger_repeats};

/// Environment marker carrying one scheduled desktop notification identifier.
pub(super) const DESKTOP_NOTIFICATION_IDENTIFIER_ENV: &str = "DESTACK_NOTIFICATION_IDENTIFIER";

/// Process-wide Linux notification launch state.
static LINUX_NOTIFICATION_LAUNCH_STATE: OnceLock<Mutex<LinuxNotificationLaunchState>> =
    OnceLock::new();

/// Process-scoped scheduled notification launch state.
#[derive(Debug)]
struct LinuxNotificationLaunchState {
    /// Scheduled notification identifier captured from the current process environment.
    marker_id: Option<String>,
    /// Launch marker initialization failure captured from the current process environment.
    marker_error: Option<String>,
    /// Runtime that already claimed the current launch marker.
    launch_runtime_id: Option<HostRuntimeId>,
}

impl Default for LinuxNotificationLaunchState {
    fn default() -> Self {
        let (marker_id, marker_error) = notification_launch_marker_state();

        Self {
            marker_id,
            marker_error,
            launch_runtime_id: None,
        }
    }
}

/// Remove Linux notification backend state for one runtime.
pub(super) fn unregister_runtime(host_runtime_id: HostRuntimeId) {
    unix::unregister_runtime(host_runtime_id);

    let state = linux_notification_launch_state();
    let mut state = state.lock();

    if state.launch_runtime_id == Some(host_runtime_id) {
        state.launch_runtime_id = None;
    }
}

/// Service Linux scheduled notification ingress for one runtime.
pub(super) fn service_notification_ingress(context: &HostRequestContext) -> RuntimeResult<()> {
    unix::service_notification_ingress(context)?;

    let marker_id = {
        let state = linux_notification_launch_state();
        let mut state = state.lock();

        if let Some(error) = state.marker_error.as_ref() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "identifier",
                format!("scheduled Linux notification launch marker failed: {error}"),
            ))
            .boxed());
        }

        if state.launch_runtime_id.is_some() {
            return Ok(());
        }

        let Some(marker_id) = state.marker_id.clone() else {
            return Ok(());
        };

        state.launch_runtime_id = Some(context.host_runtime_id);

        marker_id
    };

    let Some(record) = super::super::storage::read_notification_record(context, &marker_id)? else {
        clear_launch_runtime(context.host_runtime_id);

        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            format!(
                "destack.os.notification ingress could not find scheduled notification record for {}",
                marker_id
            ),
        ))
        .boxed());
    };

    let sequence = runtime::next_notification_sequence(context.host_runtime_id, context.platform);
    let deliver_result = unix::deliver_notification(context, &record.id, &record.request);

    if let Err(error) = deliver_result {
        clear_launch_runtime(context.host_runtime_id);
        return Err(error);
    }

    let publish_result = runtime::publish_delivered_notification(
        context.host_runtime_id,
        context.platform,
        record.id.clone(),
        record.request,
        sequence,
    );

    if let Err(error) = publish_result {
        clear_launch_runtime(context.host_runtime_id);
        return Err(error);
    }

    // one-shot timers should stop being pending after delivery succeeds
    if !notification_trigger_repeats(&record.request.trigger) {
        if let Err(error) = cancel_scheduled_notification(
            context,
            &record.id,
            false,
            "destack.os.notification.ingress",
        ) {
            clear_launch_runtime(context.host_runtime_id);
            return Err(error);
        }
    }

    Ok(())
}

/// Return the shared Linux notification launch state.
fn linux_notification_launch_state() -> &'static Mutex<LinuxNotificationLaunchState> {
    LINUX_NOTIFICATION_LAUNCH_STATE
        .get_or_init(|| Mutex::new(LinuxNotificationLaunchState::default()))
}

/// Clear the runtime that claimed the current process launch marker.
fn clear_launch_runtime(host_runtime_id: HostRuntimeId) {
    let state = linux_notification_launch_state();
    let mut state = state.lock();

    if state.launch_runtime_id == Some(host_runtime_id) {
        state.launch_runtime_id = None;
    }
}

/// Read one scheduled notification launch marker from the current process environment.
fn notification_launch_marker_state() -> (Option<String>, Option<String>) {
    match std::env::var(DESKTOP_NOTIFICATION_IDENTIFIER_ENV) {
        Ok(marker_id) if marker_id.is_empty() => (
            None,
            Some("scheduled notification identifier must not be empty".to_string()),
        ),
        Ok(marker_id) => (Some(marker_id), None),
        Err(std::env::VarError::NotPresent) => (None, None),
        Err(std::env::VarError::NotUnicode(_)) => (
            None,
            Some("scheduled notification identifier must be valid UTF-8".to_string()),
        ),
    }
}
