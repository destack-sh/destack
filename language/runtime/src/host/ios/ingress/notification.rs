use crate::diagnostic::RuntimeResult;
use crate::host::core::HostSessionHandle;
use crate::host::ios::ingress::core::ios_host_queue;
use crate::host::{HostEvent, HostNotificationEvent};
use crate::platform::os::NotificationEventValue;

/// Submit one iOS notification callback.
pub(crate) fn ios_notify_notification_event(
    session_handle: HostSessionHandle,
    event: NotificationEventValue,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Notification(Box::new(HostNotificationEvent {
        event,
    })));

    Ok(())
}
