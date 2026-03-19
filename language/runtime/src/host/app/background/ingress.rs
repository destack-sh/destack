use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::HostRuntimeId;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use super::runtime::{
    DesktopBackgroundExecutionState, background_expired_event, background_ready_event,
    desktop_background_registry, next_background_sequence, publish_background_event,
    wall_clock_now_ns,
};

/// Service background ingress for one runtime.
pub(crate) fn service_background_ingress(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
) -> RuntimeResult<()> {
    let now_unix_ns = wall_clock_now_ns()?;
    let (claimed_execution, expired_executions) = {
        let registry = desktop_background_registry();
        let mut registry = registry.lock();

        // surface invalid scheduler launch markers instead of silently discarding them
        if let Some(error) = registry.launch_marker_error.as_ref() {
            return Err(RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::InvalidArgumentValue),
                format!("destack.os.background ingress launch marker failed: {error}"),
            ))
            .boxed());
        }

        let claimed_launch_marker = if registry.launch_runtime_id.is_none() {
            registry.launch_marker.clone()
        } else {
            None
        };
        let mut claimed_execution = None;
        let mut expired_executions = Vec::new();

        // claim one pending launch marker once for the first runtime that services ingress
        if let Some(marker) = claimed_launch_marker {
            registry.launch_runtime_id = Some(host_runtime_id);
            let runtime_state = registry.runtimes.entry(host_runtime_id).or_default();
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

        let runtime_state = registry.runtimes.entry(host_runtime_id).or_default();

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

        if let Err(error) = publish_background_event(host_runtime_id, platform, event) {
            let registry = desktop_background_registry();
            let mut registry = registry.lock();

            if registry.launch_runtime_id == Some(host_runtime_id) {
                registry.launch_runtime_id = None;
            }

            return Err(error);
        }

        let registry = desktop_background_registry();
        let mut registry = registry.lock();
        let runtime_state = registry.runtimes.entry(host_runtime_id).or_default();

        runtime_state
            .executions
            .insert(execution.execution_id.clone(), execution);
    }

    // publish expirations before marking the execution expired
    for (execution, sequence) in expired_executions {
        let event = background_expired_event(execution.clone(), sequence);
        publish_background_event(host_runtime_id, platform, event)?;

        let registry = desktop_background_registry();
        let mut registry = registry.lock();

        if let Some(runtime_state) = registry.runtimes.get_mut(&host_runtime_id)
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
pub(crate) fn unregister_background_runtime(host_runtime_id: HostRuntimeId) {
    let registry = desktop_background_registry();
    let mut registry = registry.lock();

    if registry.launch_runtime_id == Some(host_runtime_id) {
        registry.launch_runtime_id = None;
    }

    registry.runtimes.remove(&host_runtime_id);
}
