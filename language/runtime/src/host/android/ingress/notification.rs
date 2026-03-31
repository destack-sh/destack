use crate::diagnostic::RuntimeResult;
use crate::host::abi::notification::{
    HostNotificationEvent as HostAbiNotificationEvent, decode_notification_event,
};
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostEvent, HostNotificationEvent};

/// Submit one Android notification callback.
pub(crate) fn android_notify_notification_event(
    session_handle: HostSessionHandle,
    event: HostAbiNotificationEvent,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;
    let event = decode_notification_event(event)?;

    queue.enqueue(HostEvent::Notification(Box::new(HostNotificationEvent {
        event,
    })));

    Ok(())
}
