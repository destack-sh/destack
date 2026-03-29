use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::{HostRequestId, HostSessionHandle};
use crate::host::{HostEvent, HostPermissionEvent};

/// Submit one Android permission-result callback.
pub(crate) fn android_notify_permission_result(
    session_handle: HostSessionHandle,
    request_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Permission(HostPermissionEvent {
        request_id: Some(HostRequestId(request_id)),
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}
