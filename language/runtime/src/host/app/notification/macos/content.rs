use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRuntimeId;
use crate::platform::os::NotificationActionStyle;
use crate::platform::os::abi_generated::{
    NotificationActionValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

use super::core::{MacosNotificationPayload, macos_notification_payload_error};

/// Build one mutable content payload for one notification request.
pub(super) fn build_notification_content(
    request: &NotificationRequestValue,
    payload: MacosNotificationPayload,
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
    let payload = serde_json::to_string(&payload).map_err(macos_notification_payload_error)?;
    let payload = NSString::from_str(&payload);
    content.setTargetContentIdentifier(Some(&payload));

    Ok(content)
}

/// Decode one pending descriptor from one native request.
pub(super) fn pending_descriptor_from_native_request(
    request: &objc2_user_notifications::UNNotificationRequest,
    host_runtime_id: HostRuntimeId,
) -> RuntimeResult<Option<NotificationScheduledDescriptorValue>> {
    let id = request.identifier().to_string();
    let payload = notification_payload_from_native_request(request)?;

    if payload.host_runtime_id != host_runtime_id.0 {
        return Ok(None);
    }

    Ok(Some(NotificationScheduledDescriptorValue {
        id,
        request: payload.request,
        scheduled_unix_ns: payload.scheduled_unix_ns,
    }))
}

/// Decode one preserved macOS notification payload from one native request.
pub(super) fn notification_payload_from_native_request(
    request: &objc2_user_notifications::UNNotificationRequest,
) -> RuntimeResult<MacosNotificationPayload> {
    let content = request.content();
    let payload = content
        .targetContentIdentifier()
        .map(|value| value.to_string())
        .unwrap_or_default();

    serde_json::from_str::<MacosNotificationPayload>(&payload)
        .map_err(macos_notification_payload_error)
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
