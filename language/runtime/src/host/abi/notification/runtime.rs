use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::host::core::error::not_supported;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::NativeAbiCodec;
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::{
    NotificationDeliveredEventValue, NotificationDismissedEventValue,
    NotificationEventMetadataValue, NotificationEventValue, NotificationImmediateTriggerValue,
    NotificationInteractedEventValue, NotificationInteractedPayloadValue, NotificationPriority,
    NotificationRequestValue, NotificationTriggerValue,
};
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::runtime::BindingCallContext;

use crate::host::abi::notification::{
    HostNotificationEvent, HostNotificationEventKind, HostNotificationRequest,
};

/// Encode one mobile notification request payload for the host ABI.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) fn encode_notification_request(
    binding: &BindingCallContext,
    request: &NotificationRequestValue,
    operation: &'static str,
) -> RuntimeResult<HostNotificationRequest> {
    // validate the required identifier first
    if request.tag.is_empty() {
        return Err(invalid_argument_value(
            "request.tag",
            "mobile notification requests require one non-empty tag",
        ));
    }

    // reject desktop-only fields that the current mobile lane does not expose
    if request.subtitle.is_some()
        || request.channel_id.is_some()
        || request.badge_count.is_some()
        || request.sound.is_some()
        || request.category_id.is_some()
        || request.thread_id.is_some()
        || request.action_id.is_some()
    {
        return Err(not_supported(operation));
    }

    // reject non-immediate triggers until the mobile lane grows a real scheduler
    if !matches!(
        request.trigger,
        NotificationTriggerValue::NotificationImmediateTrigger(_)
    ) {
        return Err(not_supported(operation));
    }

    Ok(HostNotificationRequest {
        identifier: NativeStringRef::from_value(binding, request.tag.clone()),
        title: NativeStringRef::from_value(binding, request.title.clone()),
        body: NativeStringRef::from_value(binding, request.body.clone()),
    })
}

/// Decode one mobile notification event payload from the host ABI.
pub(crate) fn decode_notification_event(
    event: HostNotificationEvent,
) -> RuntimeResult<NotificationEventValue> {
    let request = decode_notification_request(
        event.request,
        event.has_action_identifier,
        event.action_identifier,
    )?;
    let metadata = NotificationEventMetadataValue {
        timestamp_ns: event.timestamp_ns,
        sequence: event.sequence,
        id: request.tag.clone(),
        request,
    };

    Ok(match event.kind {
        HostNotificationEventKind::Delivered => {
            NotificationEventValue::NotificationDeliveredEvent(NotificationDeliveredEventValue {
                kind: "delivered".to_string(),
                metadata,
            })
        }
        HostNotificationEventKind::Activated => {
            let action_id = decode_optional_string(
                event.has_action_identifier,
                event.action_identifier,
                "HostNotificationEvent.action_identifier",
            )?;

            NotificationEventValue::NotificationInteractedEvent(NotificationInteractedEventValue {
                kind: "interacted".to_string(),
                metadata,
                payload: NotificationInteractedPayloadValue {
                    action_id,
                    action_response_text: None,
                },
            })
        }
        HostNotificationEventKind::Dismissed => {
            NotificationEventValue::NotificationDismissedEvent(NotificationDismissedEventValue {
                kind: "dismissed".to_string(),
                metadata,
            })
        }
    })
}

/// Decode one mobile notification request payload from the host ABI.
fn decode_notification_request(
    request: HostNotificationRequest,
    has_action_identifier: bool,
    action_identifier: NativeStringRef,
) -> RuntimeResult<NotificationRequestValue> {
    let identifier =
        decode_required_string(request.identifier, "HostNotificationRequest.identifier")?;
    let title = decode_required_string(request.title, "HostNotificationRequest.title")?;
    let body = decode_required_string(request.body, "HostNotificationRequest.body")?;
    let action_id = decode_optional_string(
        has_action_identifier,
        action_identifier,
        "HostNotificationEvent.action_identifier",
    )?;

    Ok(NotificationRequestValue {
        title,
        subtitle: None,
        body,
        tag: identifier,
        channel_id: None,
        priority: NotificationPriority::Normal,
        badge_count: None,
        sound: None,
        category_id: None,
        thread_id: None,
        trigger: NotificationTriggerValue::NotificationImmediateTrigger(
            NotificationImmediateTriggerValue {
                kind: "immediate".to_string(),
            },
        ),
        action_id,
    })
}

/// Decode one required notification string from the host ABI.
fn decode_required_string(value: NativeStringRef, argument: &'static str) -> RuntimeResult<String> {
    unsafe { value.as_str() }
        .map(str::to_string)
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} string")))
}

/// Decode one optional notification string from the host ABI.
fn decode_optional_string(
    is_present: bool,
    value: NativeStringRef,
    argument: &'static str,
) -> RuntimeResult<Option<String>> {
    if !is_present {
        return Ok(None);
    }

    decode_required_string(value, argument).map(Some)
}
