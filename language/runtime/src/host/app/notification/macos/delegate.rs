use tracing::warn;

use crate::host::Platform;
use crate::host::app::notification::runtime;
use crate::host::core::HostRuntimeId;
use crate::platform::os::abi_generated::NotificationInteractedPayloadValue;

use super::content::notification_payload_from_native_request;

/// Handle one native macOS notification response through the runtime event bridge.
pub(super) fn handle_notification_response(
    response: &objc2_user_notifications::UNNotificationResponse,
) {
    use objc2::runtime::AnyObject;
    use objc2_user_notifications::{
        UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
        UNTextInputNotificationResponse,
    };

    let notification = response.notification();
    let request = notification.request();
    let id = request.identifier().to_string();
    let payload = match notification_payload_from_native_request(&request) {
        Ok(payload) => payload,
        Err(error) => {
            warn!(
                ?error,
                notification_id = id,
                "failed to decode macOS notification payload"
            );
            return;
        }
    };

    let host_runtime_id = HostRuntimeId(payload.host_runtime_id);
    runtime::remove_posted_notification(host_runtime_id, &id);
    let action_identifier = response.actionIdentifier().to_string();

    let dismiss_action_identifier = unsafe { UNNotificationDismissActionIdentifier }.to_string();
    if action_identifier == dismiss_action_identifier {
        let sequence = runtime::next_notification_sequence(host_runtime_id, Platform::MacOS);
        let publish_result = runtime::publish_dismissed_notification(
            host_runtime_id,
            Platform::MacOS,
            id,
            payload.request,
            sequence,
        );

        if let Err(error) = publish_result {
            warn!(?error, "failed to publish macOS notification dismissal");
        }

        return;
    }

    let action_response_text = (response as &AnyObject)
        .downcast_ref::<UNTextInputNotificationResponse>()
        .map(|response| response.userText().to_string());
    let default_action_identifier = unsafe { UNNotificationDefaultActionIdentifier }.to_string();
    let action_id = if action_identifier == default_action_identifier {
        payload.request.action_id.clone()
    } else {
        Some(action_identifier)
    };
    let sequence = runtime::next_notification_sequence(host_runtime_id, Platform::MacOS);
    let publish_result = runtime::publish_interacted_notification(
        host_runtime_id,
        Platform::MacOS,
        id,
        payload.request,
        sequence,
        NotificationInteractedPayloadValue {
            action_id,
            action_response_text,
        },
    );

    if let Err(error) = publish_result {
        warn!(?error, "failed to publish macOS notification interaction");
    }
}
