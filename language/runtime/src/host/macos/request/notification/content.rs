use crate::diagnostic::RuntimeResult;
use crate::host::HostSessionId;
use crate::platform::os::NotificationActionStyle;
use crate::platform::os::abi_generated::{NotificationActionValue, NotificationRequestValue};

use super::core::{decode_notification_host_session_id, encode_notification_host_session_id};

/// Build one mutable content payload for one notification request.
pub(super) fn build_notification_content(
    request: &NotificationRequestValue,
    host_session_id: HostSessionId,
) -> RuntimeResult<objc2::rc::Retained<objc2_user_notifications::UNMutableNotificationContent>> {
    use objc2_foundation::{NSNumber, NSString};
    use objc2_user_notifications::{UNMutableNotificationContent, UNNotificationSound};

    let content = UNMutableNotificationContent::new();
    let title = NSString::from_str(&request.title);
    let body = NSString::from_str(&request.body);
    content.setTitle(&title);
    content.setBody(&body);

    // optional content fields
    if let Some(subtitle) = &request.subtitle {
        let subtitle = NSString::from_str(subtitle);
        content.setSubtitle(&subtitle);
    }

    if let Some(category_id) = &request.category_id {
        let category_id = NSString::from_str(category_id);
        content.setCategoryIdentifier(&category_id);
    }

    if let Some(thread_id) = &request.thread_id {
        let thread_id = NSString::from_str(thread_id);
        content.setThreadIdentifier(&thread_id);
    }

    if let Some(sound_name) = &request.sound {
        let sound_name = NSString::from_str(sound_name);
        let sound = UNNotificationSound::soundNamed(&sound_name);
        content.setSound(Some(&sound));
    } else {
        let sound = UNNotificationSound::defaultSound();
        content.setSound(Some(&sound));
    }

    if let Some(badge_count) = request.badge_count {
        let badge = NSNumber::new_u32(badge_count);
        content.setBadge(Some(&badge));
    }

    // payload
    let payload = encode_notification_host_session_id(host_session_id.0);
    let payload = NSString::from_str(&payload);
    content.setTargetContentIdentifier(Some(&payload));

    Ok(content)
}

/// Decode one preserved macOS host session id from one native request.
pub(super) fn notification_host_session_id_from_native_request(
    request: &objc2_user_notifications::UNNotificationRequest,
) -> RuntimeResult<u64> {
    let content = request.content();
    let payload = content
        .targetContentIdentifier()
        .map(|value| value.to_string())
        .unwrap_or_default();

    decode_notification_host_session_id(&payload)
}

/// Return native macOS action options for one runtime notification action.
pub(super) fn notification_action_options(
    action: &NotificationActionValue,
) -> objc2_user_notifications::UNNotificationActionOptions {
    use objc2_user_notifications::UNNotificationActionOptions;

    let mut options = UNNotificationActionOptions(0);

    if action.foreground {
        options |= UNNotificationActionOptions::Foreground;
    }

    if action.authentication_required {
        options |= UNNotificationActionOptions::AuthenticationRequired;
    }

    if action.style == NotificationActionStyle::Destructive {
        options |= UNNotificationActionOptions::Destructive;
    }

    options
}
