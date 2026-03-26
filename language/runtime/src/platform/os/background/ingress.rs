use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::HostSessionId;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use crate::platform::os::background::runtime::{
    DesktopBackgroundExecutionState, DesktopBackgroundLaunchMarker, background_expired_event,
    background_ready_event, desktop_background_runtime_service, next_background_sequence,
    publish_background_event, wall_clock_now_ns,
};

/// Service background ingress for one runtime.
pub(crate) fn service_background_ingress(
    host_session_id: HostSessionId,
    platform: Platform,
) -> RuntimeResult<()> {
    let now_unix_ns = wall_clock_now_ns()?;
    let (claimed_execution, expired_executions) = {
        let service = desktop_background_runtime_service();
        let mut registry = service.registry.lock();

        // surface invalid scheduler launch markers instead of silently discarding them
        if let Some(error) = registry.launch_marker_error.as_ref() {
            return Err(RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::InvalidArgumentValue),
                format!("destack.os.background ingress launch marker failed: {error}"),
            ))
            .boxed());
        }

        let claimed_launch_marker = if registry.launch_session_id.is_none() {
            registry.launch_marker.take()
        } else {
            None
        };
        let mut claimed_execution = None;
        let mut expired_executions = Vec::new();

        // claim one pending launch marker once for the first runtime that services ingress
        if let Some(marker) = claimed_launch_marker {
            registry.launch_session_id = Some(host_session_id);
            let runtime_state = registry.runtimes.entry(host_session_id).or_default();
            let sequence = next_background_sequence(runtime_state);

            claimed_execution = Some((
                DesktopBackgroundExecutionState {
                    identifier: marker.identifier.clone(),
                    execution_id: marker.execution_id.clone(),
                    deadline_unix_ns: marker.deadline_unix_ns,
                    is_completed: false,
                    is_expired: false,
                },
                sequence,
            ));
        }

        let runtime_state = registry.runtimes.entry(host_session_id).or_default();

        // gather expirations without mutating state before publish succeeds
        let expired_ids = runtime_state
            .executions
            .iter()
            .filter_map(|(execution_id, execution)| {
                if execution.is_completed || execution.is_expired {
                    return None;
                }

                if execution.deadline_unix_ns == 0 || execution.deadline_unix_ns > now_unix_ns {
                    return None;
                }

                Some(execution_id.clone())
            })
            .collect::<Vec<_>>();

        for execution_id in expired_ids {
            let Some(execution) = runtime_state.executions.get(&execution_id) else {
                continue;
            };
            let execution = execution.clone();
            let sequence = next_background_sequence(runtime_state);

            expired_executions.push((execution, sequence));
        }

        (claimed_execution, expired_executions)
    };

    // publish a claimed launch marker before recording runtime execution state
    if let Some((execution, sequence)) = claimed_execution {
        let event = background_ready_event(execution.clone(), sequence);

        if let Err(error) = publish_background_event(host_session_id, platform, event) {
            let service = desktop_background_runtime_service();
            let mut registry = service.registry.lock();

            if registry.launch_session_id == Some(host_session_id) {
                registry.launch_session_id = None;
                registry.launch_marker = Some(DesktopBackgroundLaunchMarker {
                    identifier: execution.identifier,
                    execution_id: execution.execution_id,
                    deadline_unix_ns: execution.deadline_unix_ns,
                });
            }

            return Err(error);
        }

        let service = desktop_background_runtime_service();
        let mut registry = service.registry.lock();
        let runtime_state = registry.runtimes.entry(host_session_id).or_default();

        runtime_state
            .executions
            .insert(execution.execution_id.clone(), execution);
    }

    // publish expirations before marking the execution expired
    for (execution, sequence) in expired_executions {
        let event = background_expired_event(execution.clone(), sequence);
        publish_background_event(host_session_id, platform, event)?;

        let service = desktop_background_runtime_service();
        let mut registry = service.registry.lock();

        if let Some(runtime_state) = registry.runtimes.get_mut(&host_session_id)
            && let Some(active_execution) =
                runtime_state.executions.get_mut(&execution.execution_id)
            && !active_execution.is_completed
        {
            active_execution.is_expired = true;
        }
    }

    Ok(())
}

/// Remove background state for one runtime id.
pub(crate) fn unregister_background_runtime(host_session_id: HostSessionId) {
    let service = desktop_background_runtime_service();
    let mut registry = service.registry.lock();

    if registry.launch_session_id == Some(host_session_id) {
        registry.launch_session_id = None;
    }

    registry.runtimes.remove(&host_session_id);
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::host::core::{HostQueue, HostSessionRegistry};
    use crate::host::{HostEvent, Platform};

    use super::{
        DesktopBackgroundLaunchMarker, RuntimeResult, service_background_ingress,
        unregister_background_runtime,
    };
    use crate::platform::os::background::runtime::desktop_background_runtime_service;

    /// Return the shared mutex that serializes background ingress tests.
    fn background_ingress_test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Reset the shared desktop background registry for one isolated test.
    fn reset_background_registry() {
        let service = desktop_background_runtime_service();
        let mut registry = service.registry.lock();

        registry.launch_marker = None;
        registry.launch_marker_error = None;
        registry.launch_session_id = None;
        registry.runtimes.clear();
    }

    /// Poll one queue and return the queued host events.
    fn poll_events(queue: &HostQueue) -> RuntimeResult<Vec<HostEvent>> {
        queue.poll_events(Some(0))
    }

    /// Consume one launch marker only once across runtime unregister and reattach.
    #[test]
    fn test_service_background_ingress_consumes_launch_marker_once() {
        let _guard = background_ingress_test_lock()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        reset_background_registry();

        let service = desktop_background_runtime_service();
        {
            let mut registry = service.registry.lock();
            registry.launch_marker = Some(DesktopBackgroundLaunchMarker {
                identifier: "sync".to_string(),
                execution_id: "execution-1".to_string(),
                deadline_unix_ns: 42,
            });
        }

        let first_runtime_id = HostSessionRegistry::allocate_session_id();
        let first_queue = Arc::new(HostQueue::new(first_runtime_id));
        let first_registration = HostSessionRegistry::register_queue(
            Platform::Linux,
            first_runtime_id,
            Arc::clone(&first_queue),
            Some(unregister_background_runtime),
        );

        service_background_ingress(first_runtime_id, Platform::Linux)
            .expect("first runtime should receive one launch marker");
        let first_events = poll_events(&first_queue).expect("first queue should poll");

        assert_eq!(first_events.len(), 1);

        drop(first_registration);

        let second_runtime_id = HostSessionRegistry::allocate_session_id();
        let second_queue = Arc::new(HostQueue::new(second_runtime_id));
        let _second_registration = HostSessionRegistry::register_queue(
            Platform::Linux,
            second_runtime_id,
            Arc::clone(&second_queue),
            Some(unregister_background_runtime),
        );

        service_background_ingress(second_runtime_id, Platform::Linux)
            .expect("second runtime should service ingress");
        let second_events = poll_events(&second_queue).expect("second queue should poll");

        assert!(second_events.is_empty());
        reset_background_registry();
    }
}
