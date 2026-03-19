use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostNotificationEvent, HostRuntimeId, HostRuntimeRegistry};
use crate::host::{HostEvent, Platform};
use crate::platform::core::monotonic_now_ns;
use crate::platform::os::abi_generated::{
    NotificationDeliveredEventValue, NotificationEventMetadataValue, NotificationEventValue,
    NotificationRequestValue,
};
#[cfg(any(target_os = "macos", windows, all(unix, not(target_os = "macos"))))]
use crate::platform::os::abi_generated::{
    NotificationDismissedEventValue, NotificationInteractedEventValue,
    NotificationInteractedPayloadValue,
};

use super::state::{DesktopNotificationRuntimeState, notification_runtime_service};

/// Publish one delivered notification event into the runtime host queue.
pub(in crate::host::app::notification) fn publish_delivered_notification(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    id: String,
    request: NotificationRequestValue,
    sequence: u64,
) -> RuntimeResult<()> {
    let event =
        NotificationEventValue::NotificationDeliveredEvent(NotificationDeliveredEventValue {
            kind: "delivered".to_string(),
            metadata: notification_event_metadata(id, request, sequence),
        });

    publish_notification_event(host_runtime_id, platform, event)
}

/// Publish one interacted notification event into the runtime host queue.
#[cfg(any(target_os = "macos", windows, all(unix, not(target_os = "macos"))))]
pub(in crate::host::app::notification) fn publish_interacted_notification(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    id: String,
    request: NotificationRequestValue,
    sequence: u64,
    payload: NotificationInteractedPayloadValue,
) -> RuntimeResult<()> {
    let event =
        NotificationEventValue::NotificationInteractedEvent(NotificationInteractedEventValue {
            kind: "interacted".to_string(),
            metadata: notification_event_metadata(id, request, sequence),
            payload,
        });

    publish_notification_event(host_runtime_id, platform, event)
}

/// Publish one dismissed notification event into the runtime host queue.
#[cfg(any(target_os = "macos", windows, all(unix, not(target_os = "macos"))))]
pub(in crate::host::app::notification) fn publish_dismissed_notification(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    id: String,
    request: NotificationRequestValue,
    sequence: u64,
) -> RuntimeResult<()> {
    let event =
        NotificationEventValue::NotificationDismissedEvent(NotificationDismissedEventValue {
            kind: "dismissed".to_string(),
            metadata: notification_event_metadata(id, request, sequence),
        });

    publish_notification_event(host_runtime_id, platform, event)
}

/// Allocate the next notification event sequence for one runtime.
#[cfg(any(target_os = "macos", windows, all(unix, not(target_os = "macos"))))]
pub(in crate::host::app::notification) fn next_notification_sequence(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
) -> u64 {
    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(host_runtime_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(platform));

    runtime_state.next_sequence()
}

/// Build shared metadata for one notification event.
fn notification_event_metadata(
    id: String,
    request: NotificationRequestValue,
    sequence: u64,
) -> NotificationEventMetadataValue {
    NotificationEventMetadataValue {
        timestamp_ns: monotonic_now_ns(),
        sequence,
        id,
        request,
    }
}

/// Publish one notification event into the runtime host queue.
fn publish_notification_event(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    event: NotificationEventValue,
) -> RuntimeResult<()> {
    let queue = HostRuntimeRegistry::queue_for_runtime(host_runtime_id, platform)?;

    queue.enqueue(HostEvent::Notification(Box::new(HostNotificationEvent {
        event,
    })));
    queue.poll_wake_handle().wake()?;

    Ok(())
}
