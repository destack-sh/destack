use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::{HostBackgroundEvent, HostEvent, HostRuntimeId, HostRuntimeRegistry};
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
pub(in crate::host::app::background) fn complete_execution(
    host_runtime_id: HostRuntimeId,
    execution_id: &str,
) -> RuntimeResult<()> {
    let service = desktop_background_runtime_service();
    let mut registry = service.registry.lock();
    let Some(runtime_state) = registry.runtimes.get_mut(&host_runtime_id) else {
        return Err(not_supported("destack.os.background.complete"));
    };
    let Some(execution) = runtime_state.executions.get_mut(execution_id) else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            "background execution not found",
        ))
        .boxed());
    };

    execution.is_completed = true;

    Ok(())
}

/// Return the next background event sequence for one runtime.
pub(in crate::host::app::background) fn next_background_sequence(
    runtime_state: &mut DesktopBackgroundRuntimeState,
) -> u64 {
    let sequence = runtime_state.next_sequence;
    runtime_state.next_sequence = runtime_state.next_sequence.wrapping_add(1);

    sequence
}

/// Build one task-ready background event.
pub(in crate::host::app::background) fn background_ready_event(
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
pub(in crate::host::app::background) fn background_expired_event(
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
pub(in crate::host::app::background) fn publish_background_event(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    event: BackgroundEventValue,
) -> RuntimeResult<()> {
    let queue = HostRuntimeRegistry::queue_for_runtime(host_runtime_id, platform)?;

    queue.enqueue(HostEvent::Background(Box::new(HostBackgroundEvent {
        event,
    })));
    queue.poll_wake_handle().wake()?;

    Ok(())
}
