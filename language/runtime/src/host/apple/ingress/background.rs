use crate::diagnostic::RuntimeResult;
use crate::host::abi::background::{HostBackgroundEvent as HostAbiBackgroundEvent, decode_event};
use crate::host::apple::ingress::core::ios_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostBackgroundEvent, HostEvent};

/// Submit one iOS background callback.
pub(crate) fn ios_notify_background_event(
    session_handle: HostSessionHandle,
    event: HostAbiBackgroundEvent,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;
    let event = decode_event(event)?;

    queue.enqueue(HostEvent::Background(Box::new(HostBackgroundEvent {
        event,
    })));

    Ok(())
}
