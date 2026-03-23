use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostEvent, HostNotificationEvent};
use crate::platform::os::NotificationEventValue;

/// Submit one Android notification callback.
pub(crate) fn android_notify_notification_event(
    session_handle: HostSessionHandle,
    event: NotificationEventValue,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Notification(Box::new(HostNotificationEvent {
        event,
    })));

    Ok(())
}
