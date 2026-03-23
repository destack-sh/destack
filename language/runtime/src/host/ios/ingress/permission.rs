use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequestId, HostSessionHandle};
use crate::host::ios::ingress::core::ios_host_queue;
use crate::host::{HostEvent, HostPermissionEvent};

/// Submit one iOS permission-result callback.
pub(crate) fn ios_notify_permission_result(
    session_handle: HostSessionHandle,
    request_id: Option<u64>,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Permission(HostPermissionEvent {
        request_id: request_id.map(HostRequestId),
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}
