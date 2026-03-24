use tracing::warn;

use crate::host::Platform;
use crate::host::core::HostSessionId;
use crate::platform::os::abi_generated::NotificationInteractedPayloadValue;
use crate::platform::os::notification::runtime;

use super::content::notification_host_session_id_from_native_request;

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
    let host_session_id = match notification_host_session_id_from_native_request(&request) {
        Ok(host_session_id) => HostSessionId(host_session_id),
        Err(error) => {
            warn!(
                ?error,
                notification_id = id,
                "failed to decode macOS notification host session id"
            );
            return;
        }
    };

    let Some(notification_request) = runtime::notification_request(host_session_id, &id) else {
        warn!(
            notification_id = id,
            "failed to resolve macOS notification request"
        );
        return;
    };

    runtime::remove_notification_request(host_session_id, &id);
    let action_identifier = response.actionIdentifier().to_string();

    let dismiss_action_identifier = unsafe { UNNotificationDismissActionIdentifier }.to_string();
    if action_identifier == dismiss_action_identifier {
        let sequence = runtime::next_notification_sequence(host_session_id, Platform::MacOS);
        let publish_result = runtime::publish_dismissed_notification(
            host_session_id,
            Platform::MacOS,
            id,
            notification_request,
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
        notification_request.action_id.clone()
    } else {
        Some(action_identifier)
    };
    let sequence = runtime::next_notification_sequence(host_session_id, Platform::MacOS);
    let publish_result = runtime::publish_interacted_notification(
        host_session_id,
        Platform::MacOS,
        id,
        notification_request,
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
