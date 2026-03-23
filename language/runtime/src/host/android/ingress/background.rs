use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostBackgroundEvent, HostEvent};
use crate::platform::os::BackgroundEventValue;

/// Submit one Android background callback.
pub(crate) fn android_notify_background_event(
    session_handle: HostSessionHandle,
    event: BackgroundEventValue,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Background(Box::new(HostBackgroundEvent {
        event,
    })));

    Ok(())
}
