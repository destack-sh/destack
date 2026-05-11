use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::os::unix::request::notification::core as unix_notification;
use crate::host::{HostSessionId, RequestContext, SessionContext};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::notification::runtime;
use crate::platform::os::notification::storage::read_notification_record;
use crate::platform::os::notification::wrapper::DESKTOP_NOTIFICATION_IDENTIFIER_ENV;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

use super::schedule::{cancel_scheduled_notification, notification_trigger_repeats};

/// Process-scoped scheduled notification launch state.
#[derive(Debug)]
struct LinuxNotificationLaunchState {
    /// Scheduled notification identifier captured from the current process environment.
    marker_id: Option<String>,
    /// Launch marker initialization failure captured from the current process environment.
    marker_error: Option<String>,
    /// Runtime that already claimed the current launch marker.
    launch_session_id: Option<HostSessionId>,
}

impl Default for LinuxNotificationLaunchState {
    fn default() -> Self {
        let (marker_id, marker_error) = notification_launch_marker_state();

        Self {
            marker_id,
            marker_error,
            launch_session_id: None,
        }
    }
}

/// Process-global Linux notification launch service.
struct LinuxNotificationLaunchService {
    /// Process-wide Linux notification launch state.
    state: Mutex<LinuxNotificationLaunchState>,
}

impl LinuxNotificationLaunchService {
    /// Create one Linux notification launch service.
    fn new() -> Self {
        Self {
            state: Mutex::new(LinuxNotificationLaunchState::default()),
        }
    }
}

impl Service for LinuxNotificationLaunchService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Remove Linux notification backend state for one runtime.
pub(crate) fn unregister_runtime(host_session_id: HostSessionId) {
    unix_notification::unregister_runtime(host_session_id);

    let service = linux_notification_launch_service();
    let mut state = service.state.lock();

    if state.launch_session_id == Some(host_session_id) {
        state.launch_session_id = None;
    }
}

/// Service Linux scheduled notification ingress for one runtime.
pub(crate) fn service_notification_ingress(context: &SessionContext) -> RuntimeResult<()> {
    unix_notification::service_notification_ingress(context)?;

    let marker_id = {
        let service = linux_notification_launch_service();
        let mut state = service.state.lock();

        if let Some(error) = state.marker_error.as_ref() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "identifier",
                format!("scheduled Linux notification launch marker failed: {error}"),
            ))
            .boxed());
        }

        if state.launch_session_id.is_some() {
            return Ok(());
        }

        let Some(marker_id) = state.marker_id.clone() else {
            return Ok(());
        };

        state.launch_session_id = Some(context.host_session_id);

        marker_id
    };

    let Some(record) = read_notification_record(context, &marker_id)? else {
        clear_launch_runtime(context.host_session_id);

        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            format!(
                "destack.os.notification ingress could not find scheduled notification record for {}",
                marker_id
            ),
        ))
        .boxed());
    };

    let sequence = runtime::next_notification_sequence(context.host_session_id, context.platform);
    let deliver_result =
        unix_notification::deliver_notification(context, &record.id, &record.request);

    if let Err(error) = deliver_result {
        clear_launch_runtime(context.host_session_id);
        return Err(error);
    }

    let publish_result = runtime::publish_delivered_notification(
        context.host_session_id,
        context.platform,
        record.id.clone(),
        record.request,
        sequence,
    );

    if let Err(error) = publish_result {
        clear_launch_runtime(context.host_session_id);
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
            clear_launch_runtime(context.host_session_id);
            return Err(error);
        }
    }

    Ok(())
}

/// Return the shared Linux notification launch service.
fn linux_notification_launch_service() -> Arc<LinuxNotificationLaunchService> {
    match LinuxNotificationLaunchService::global(|| Ok(LinuxNotificationLaunchService::new())) {
        Ok(service) => service,
        Err(error) => {
            panic!("linux notification launch service should be infallible: {error}");
        }
    }
}

/// Clear the runtime that claimed the current process launch marker.
fn clear_launch_runtime(host_session_id: HostSessionId) {
    let service = linux_notification_launch_service();
    let mut state = service.state.lock();

    if state.launch_session_id == Some(host_session_id) {
        state.launch_session_id = None;
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
