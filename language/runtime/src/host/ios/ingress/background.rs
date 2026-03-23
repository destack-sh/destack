use crate::diagnostic::RuntimeResult;
use crate::host::core::HostSessionHandle;
use crate::host::ios::ingress::core::ios_host_queue;
use crate::host::{HostBackgroundEvent, HostEvent};
use crate::platform::os::BackgroundEventValue;

/// Submit one iOS background callback.
pub(crate) fn ios_notify_background_event(
    session_handle: HostSessionHandle,
    event: BackgroundEventValue,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Background(Box::new(HostBackgroundEvent {
        event,
    })));

    Ok(())
}
