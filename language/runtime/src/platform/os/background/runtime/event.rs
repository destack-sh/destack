use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostBackgroundEvent, HostEvent, HostSessionId, HostSessionRegistry, Platform};
use crate::platform::PlatformError;
use crate::platform::core::{monotonic_now_ns, not_supported};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundEventMetadataValue, BackgroundEventValue, BackgroundTaskExpiredEventValue,
    BackgroundTaskReadyEventValue,
};

use super::state::{
    DesktopBackgroundExecutionState, DesktopBackgroundRuntimeState,
    desktop_background_runtime_service,
};

/// Mark one active desktop background execution as complete.
pub(crate) fn complete_execution(
    host_session_id: HostSessionId,
    execution_id: &str,
) -> RuntimeResult<DesktopBackgroundExecutionState> {
    let service = desktop_background_runtime_service();
    let mut registry = service.registry.lock();
    let Some(runtime_state) = registry.runtimes.get_mut(&host_session_id) else {
        return Err(not_supported("destack.os.background.complete"));
    };
    let Some(execution) = runtime_state.executions.get_mut(execution_id) else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            "background execution not found",
        ))
        .boxed());
    };

    // reject duplicate completion reports for one finalized execution
    if execution.is_completed {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            "background execution already completed",
        ))
        .boxed());
    }

    execution.is_completed = true;

    Ok(execution.clone())
}

/// Return the next background event sequence for one runtime.
pub(crate) fn next_background_sequence(runtime_state: &mut DesktopBackgroundRuntimeState) -> u64 {
    let sequence = runtime_state.next_sequence;
    runtime_state.next_sequence = runtime_state.next_sequence.wrapping_add(1);

    sequence
}

/// Build one task-ready background event.
pub(crate) fn background_ready_event(
    execution: DesktopBackgroundExecutionState,
    sequence: u64,
) -> BackgroundEventValue {
    BackgroundEventValue::BackgroundTaskReadyEvent(BackgroundTaskReadyEventValue {
        kind: "taskReady".to_string(),
        metadata: BackgroundEventMetadataValue {
            timestamp_ns: monotonic_now_ns(),
            sequence,
            identifier: execution.identifier,
            execution_id: execution.execution_id,
            deadline_unix_ns: execution.deadline_unix_ns,
        },
    })
}

/// Build one task-expired background event.
pub(crate) fn background_expired_event(
    execution: DesktopBackgroundExecutionState,
    sequence: u64,
) -> BackgroundEventValue {
    BackgroundEventValue::BackgroundTaskExpiredEvent(BackgroundTaskExpiredEventValue {
        kind: "taskExpired".to_string(),
        metadata: BackgroundEventMetadataValue {
            timestamp_ns: monotonic_now_ns(),
            sequence,
            identifier: execution.identifier,
            execution_id: execution.execution_id,
            deadline_unix_ns: execution.deadline_unix_ns,
        },
    })
}

/// Publish one desktop background event into the runtime host queue.
pub(crate) fn publish_background_event(
    host_session_id: HostSessionId,
    platform: Platform,
    event: BackgroundEventValue,
) -> RuntimeResult<()> {
    let queue = HostSessionRegistry::queue_for_session(host_session_id, platform)?;

    queue.enqueue(HostEvent::Background(Box::new(HostBackgroundEvent {
        event,
    })));
    queue.poll_wake_handle().wake()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::host::HostSessionId;
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::tests::platform::assert_runtime_error_code;

    use super::complete_execution;
    use crate::platform::os::background::runtime::{
        DesktopBackgroundExecutionState, desktop_background_runtime_service,
    };

    /// Reject duplicate background completion reports for one execution token.
    #[test]
    fn test_complete_execution_rejects_duplicate_completion() {
        let service = desktop_background_runtime_service();
        let mut registry = service.registry.lock();
        let host_session_id = HostSessionId(1);

        registry.runtimes.clear();
        registry
            .runtimes
            .entry(host_session_id)
            .or_default()
            .executions
            .insert(
                "execution-1".to_string(),
                DesktopBackgroundExecutionState {
                    identifier: "sync".to_string(),
                    execution_id: "execution-1".to_string(),
                    deadline_unix_ns: 42,
                    is_completed: false,
                    is_expired: false,
                },
            );
        drop(registry);

        complete_execution(host_session_id, "execution-1").expect("first completion should work");

        let error = complete_execution(host_session_id, "execution-1")
            .expect_err("duplicate completion should fail");

        assert_runtime_error_code(&error, PlatformErrorCode::IoNotFound);
    }
}
