use crate::diagnostic::RuntimeResult;
use crate::host::abi::permission::HostPermissionEvent as HostAbiPermissionEvent;
use crate::host::apple::ingress::core::ios_host_queue;
use crate::host::core::error::invalid_argument_value;
use crate::host::core::{HostRequestId, HostSessionHandle};
use crate::host::{HostEvent, HostPermissionEvent};

/// Submit one iOS permission-result callback.
pub(crate) fn ios_notify_permission_result(
    session_handle: HostSessionHandle,
    event: HostAbiPermissionEvent,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;
    let permission = unsafe { event.permission.as_str() }.map_err(|_| {
        invalid_argument_value(
            "permission",
            "invalid HostPermissionEvent.permission string",
        )
    })?;

    let event = HostPermissionEvent {
        request_id: Some(HostRequestId(event.request_id)),
        permission: permission.to_string(),
        granted: event.is_granted,
    };

    queue.enqueue(HostEvent::Permission(event));

    Ok(())
}
