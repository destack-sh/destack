use windows::Win32::UI::Notifications::NOTIFICATION_USER_INPUT_DATA;
use windows_core::Interface;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostSessionId;
use crate::host::core::error::not_supported;
use crate::platform::os::abi_generated::{
    NotificationActionValue, NotificationCategoryValue, NotificationInteractedPayloadValue,
    NotificationRequestValue, NotificationTimeIntervalTriggerValue, NotificationTriggerValue,
};
use crate::platform::os::notification::runtime;
use crate::platform::os::{NotificationActionStyle, NotificationPriority};

use super::activation::WindowsNotificationActivationPayload;
use super::core::{
    NANOSECONDS_PER_TICK, WINDOWS_EPOCH_OFFSET_TICKS, windows_notification_error,
    windows_notification_payload_error,
};
use crate::platform::os::notification::time::calendar_date_trigger_unix_ns;

/// Remove one scheduled Windows notification by identifier when present.
pub(super) fn remove_scheduled_notification_by_id(
    notifier: &windows::UI::Notifications::ToastNotifier,
    id: &str,
) -> RuntimeResult<()> {
    let scheduled = notifier
        .GetScheduledToastNotifications()
        .map_err(windows_notification_error)?;
    let count = scheduled.Size().map_err(windows_notification_error)?;

    for index in 0..count {
        let scheduled = scheduled.GetAt(index).map_err(windows_notification_error)?;
        let scheduled_id = scheduled
            .Id()
            .map_err(windows_notification_error)?
            .to_string_lossy();

        if scheduled_id == id {
            notifier
                .RemoveFromSchedule(&scheduled)
                .map_err(windows_notification_error)?;
            break;
        }
    }

    Ok(())
}

/// Build one Windows toast document for one request.
pub(super) fn windows_toast_document(
    host_session_id: HostSessionId,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<windows::Data::Xml::Dom::XmlDocument> {
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::core::HSTRING;

    let xml = windows_toast_xml(host_session_id, id, request)?;
    let document = XmlDocument::new().map_err(windows_notification_error)?;
    document
        .LoadXml(&HSTRING::from(xml))
        .map_err(windows_notification_error)?;

    Ok(document)
}

/// Build one Windows toast xml payload.
pub(super) fn windows_toast_xml(
    host_session_id: HostSessionId,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<String> {
    let title = windows_notification_xml_escape(&request.title);
    let subtitle = request.subtitle.as_deref().unwrap_or_default();
    let subtitle = windows_notification_xml_escape(subtitle);
    let body = windows_notification_xml_escape(&request.body);
    let launch = windows_notification_activation_payload(
        host_session_id,
        id,
        request,
        request.action_id.as_deref(),
        None,
    )?;
    let launch = windows_notification_xml_escape(&launch);
    let header = if request.tag.is_empty() {
        id
    } else {
        &request.tag
    };
    let header = windows_notification_xml_escape(header);
    let actions = windows_notification_actions_xml(host_session_id, id, request)?;
    let use_button_style = if actions.uses_button_style {
        " useButtonStyle=\"true\""
    } else {
        ""
    };
    let actions_xml = if actions.xml.is_empty() {
        String::new()
    } else {
        format!("<actions>{}</actions>", actions.xml)
    };

    if subtitle.is_empty() {
        return Ok(format!(
            "<toast launch=\"{launch}\"{use_button_style}><visual><binding template=\"ToastGeneric\"><text>{title}</text><text>{body}</text></binding></visual><header id=\"{header}\" title=\"{title}\" arguments=\"{launch}\"/>{actions_xml}</toast>"
        ));
    }

    Ok(format!(
        "<toast launch=\"{launch}\"{use_button_style}><visual><binding template=\"ToastGeneric\"><text>{title}</text><text>{subtitle}</text><text>{body}</text></binding></visual><header id=\"{header}\" title=\"{title}\" arguments=\"{launch}\"/>{actions_xml}</toast>"
    ))
}

/// Windows action payload fragments for one toast notification.
pub(super) struct WindowsNotificationActions {
    /// The rendered actions xml fragment.
    pub(super) xml: String,
    /// Whether any rendered action uses button-style hints.
    pub(super) uses_button_style: bool,
}

/// Build one Windows actions payload for the request category when present.
pub(super) fn windows_notification_actions_xml(
    host_session_id: HostSessionId,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<WindowsNotificationActions> {
    let Some(category_id) = request.category_id.as_deref() else {
        return Ok(WindowsNotificationActions {
            xml: String::new(),
            uses_button_style: false,
        });
    };
    let category = runtime::notification_category(host_session_id, category_id)?;

    let Some(category) = category else {
        return Ok(WindowsNotificationActions {
            xml: String::new(),
            uses_button_style: false,
        });
    };

    let mut actions_xml = String::new();
    let mut uses_button_style = false;

    for (index, action) in category.actions.iter().enumerate() {
        if action.style != NotificationActionStyle::TextInput {
            continue;
        }

        let input_id = windows_notification_text_input_id(index);
        let placeholder = action.text_input_placeholder.as_deref().unwrap_or("Reply");
        let placeholder = windows_notification_xml_escape(placeholder);

        actions_xml.push_str(&format!(
            "<input id=\"{input_id}\" type=\"text\" placeHolderContent=\"{placeholder}\"/>"
        ));
    }

    for (index, action) in category.actions.iter().enumerate() {
        let activation = windows_notification_activation_payload(
            host_session_id,
            id,
            request,
            Some(action.id.as_str()),
            windows_notification_input_id_for_action(index, action).as_deref(),
        )?;
        let activation = windows_notification_xml_escape(&activation);
        let title = if action.style == NotificationActionStyle::TextInput {
            action
                .text_input_button_title
                .as_deref()
                .unwrap_or(action.title.as_str())
        } else {
            action.title.as_str()
        };
        let title = windows_notification_xml_escape(title);
        let activation_type = windows_notification_activation_type(action);
        let input_id = windows_notification_input_id_for_action(index, action);
        let input_id = input_id
            .as_deref()
            .map(|input_id| format!(" hint-inputId=\"{input_id}\""))
            .unwrap_or_default();
        let button_style = windows_notification_button_style(action);
        let button_style = button_style
            .map(|button_style| format!(" hint-buttonStyle=\"{button_style}\""))
            .unwrap_or_default();

        if !button_style.is_empty() {
            uses_button_style = true;
        }

        actions_xml.push_str(&format!(
            "<action content=\"{title}\" arguments=\"{activation}\" activationType=\"{activation_type}\"{input_id}{button_style}/>"
        ));
    }

    Ok(WindowsNotificationActions {
        xml: actions_xml,
        uses_button_style,
    })
}

/// Convert one notification trigger into one Windows delivery timestamp.
pub(super) fn windows_scheduled_delivery_time(
    trigger: &NotificationTriggerValue,
) -> RuntimeResult<windows::Foundation::DateTime> {
    let unix_ns = trigger_delivery_unix_ns(trigger)?;

    Ok(unix_ns_to_windows_datetime(unix_ns))
}

/// Convert one Unix nanosecond timestamp into one Windows delivery timestamp.
fn unix_ns_to_windows_datetime(unix_ns: u64) -> windows::Foundation::DateTime {
    let ticks = (unix_ns / NANOSECONDS_PER_TICK).min(i64::MAX as u64);

    windows::Foundation::DateTime {
        UniversalTime: WINDOWS_EPOCH_OFFSET_TICKS.saturating_add(ticks as i64),
    }
}

/// Return the wall-clock delivery timestamp for one notification trigger.
fn trigger_delivery_unix_ns(trigger: &NotificationTriggerValue) -> RuntimeResult<u64> {
    match trigger {
        NotificationTriggerValue::NotificationImmediateTrigger(_) => {
            Err(not_supported("destack.os.notification.schedule"))
        }
        NotificationTriggerValue::NotificationTimeIntervalTrigger(value) => {
            time_interval_trigger_unix_ns(value)
        }
        NotificationTriggerValue::NotificationCalendarDateTrigger(value) => {
            calendar_date_trigger_unix_ns(value)
        }
    }
}

/// Return the wall-clock delivery timestamp for one time-interval trigger.
fn time_interval_trigger_unix_ns(
    value: &NotificationTimeIntervalTriggerValue,
) -> RuntimeResult<u64> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(windows_notification_time_error)?;
    let now_unix_ns = now.as_nanos().min(u128::from(u64::MAX)) as u64;

    Ok(now_unix_ns.saturating_add(value.interval_ns))
}

/// Map one wall-clock conversion error into one runtime error.
fn windows_notification_time_error(error: impl std::fmt::Display) -> Box<RuntimeError> {
    super::core::windows_notification_payload_error(format!(
        "Windows notification trigger time conversion failed: {error}"
    ))
}

/// Decode one Windows interaction payload from one toast activation callback.
pub(super) fn windows_interacted_payload(
    args: Option<&windows::core::IInspectable>,
    host_session_id: HostSessionId,
    id: &str,
    request: &NotificationRequestValue,
) -> RuntimeResult<NotificationInteractedPayloadValue> {
    let Some(args) = args else {
        return Ok(NotificationInteractedPayloadValue {
            action_id: request.action_id.clone(),
            action_response_text: None,
        });
    };
    let args = args
        .cast::<windows::UI::Notifications::ToastActivatedEventArgs>()
        .map_err(windows_notification_error)?;
    let arguments = args.Arguments().map_err(windows_notification_error)?;
    let arguments = arguments.to_string_lossy();
    let activation = windows_notification_activation_from_arguments(
        &arguments,
        Some(host_session_id),
        Some(id),
        Some(request),
    )?;
    let response_text = windows_notification_response_text(&args, activation.input_id.as_deref())?;

    Ok(NotificationInteractedPayloadValue {
        action_id: activation.action_id,
        action_response_text: response_text,
    })
}

/// Decode one activation payload from one Windows argument string.
pub(super) fn windows_notification_activation_from_arguments(
    arguments: &str,
    default_host_runtime_id: Option<HostSessionId>,
    default_notification_id: Option<&str>,
    default_request: Option<&NotificationRequestValue>,
) -> RuntimeResult<WindowsNotificationActivationPayload> {
    if arguments.is_empty() {
        let default_request = default_request.ok_or_else(|| {
            windows_notification_payload_error(
                "Windows activation arguments were empty without a live notification context",
            )
        })?;
        let default_host_runtime_id = default_host_runtime_id.ok_or_else(|| {
            windows_notification_payload_error(
                "Windows activation arguments were missing one runtime identifier",
            )
        })?;
        let default_notification_id = default_notification_id.ok_or_else(|| {
            windows_notification_payload_error(
                "Windows activation arguments were missing one notification identifier",
            )
        })?;

        return Ok(WindowsNotificationActivationPayload {
            source_host_runtime_id: default_host_runtime_id,
            notification_id: default_notification_id.to_string(),
            action_id: default_request.action_id.clone(),
            input_id: None,
        });
    }

    decode_windows_notification_activation_token(arguments)
}

/// Read one text-input response payload from one Windows activation callback.
pub(super) fn windows_notification_response_text(
    args: &windows::UI::Notifications::ToastActivatedEventArgs,
    input_id: Option<&str>,
) -> RuntimeResult<Option<String>> {
    let Some(input_id) = input_id else {
        return Ok(None);
    };
    let input_id = windows::core::HSTRING::from(input_id);
    let user_input = args.UserInput().map_err(windows_notification_error)?;
    let has_key = user_input
        .HasKey(&input_id)
        .map_err(windows_notification_error)?;

    if !has_key {
        return Ok(None);
    }

    let value = user_input
        .Lookup(&input_id)
        .map_err(windows_notification_error)?;
    let value = value
        .cast::<windows::Foundation::IPropertyValue>()
        .map_err(windows_notification_error)?;
    let value = value.GetString().map_err(windows_notification_error)?;

    Ok(Some(value.to_string_lossy()))
}

/// Encode one Windows activation payload as one action-argument string.
pub(super) fn windows_notification_activation_payload(
    host_session_id: HostSessionId,
    id: &str,
    _request: &NotificationRequestValue,
    action_id: Option<&str>,
    input_id: Option<&str>,
) -> RuntimeResult<String> {
    let mut payload = format!(
        "destack.notification.windows.v1\nsession={}\nid={}",
        host_session_id.0,
        encode_windows_notification_token_string(id),
    );

    if let Some(action_id) = action_id {
        payload.push_str("\naction=");
        payload.push_str(&encode_windows_notification_token_string(action_id));
    }

    if let Some(input_id) = input_id {
        payload.push_str("\ninput=");
        payload.push_str(&encode_windows_notification_token_string(input_id));
    }

    Ok(payload)
}

/// Decode one Windows activation token from one toast argument string.
fn decode_windows_notification_activation_token(
    payload: &str,
) -> RuntimeResult<WindowsNotificationActivationPayload> {
    let mut lines = payload.lines();
    let Some(kind) = lines.next() else {
        return Err(windows_notification_payload_error(
            "Windows activation arguments were empty",
        ));
    };

    if kind != "destack.notification.windows.v1" {
        return Err(windows_notification_payload_error(
            "Windows activation arguments had one unknown payload kind",
        ));
    }

    let mut source_host_runtime_id = None;
    let mut notification_id = None;
    let mut action_id = None;
    let mut input_id = None;

    for line in lines {
        let Some((key, value)) = line.split_once('=') else {
            return Err(windows_notification_payload_error(
                "Windows activation arguments had one invalid field",
            ));
        };

        match key {
            "session" => {
                let value = value
                    .parse::<u64>()
                    .map_err(windows_notification_payload_error)?;
                source_host_runtime_id = Some(value);
            }
            "id" => {
                notification_id = Some(decode_windows_notification_token_string(value)?);
            }
            "action" => {
                action_id = Some(decode_windows_notification_token_string(value)?);
            }
            "input" => {
                input_id = Some(decode_windows_notification_token_string(value)?);
            }
            _ => {
                return Err(windows_notification_payload_error(
                    "Windows activation arguments had one unknown field",
                ));
            }
        }
    }

    let source_host_runtime_id = source_host_runtime_id.ok_or_else(|| {
        windows_notification_payload_error(
            "Windows activation arguments were missing one runtime identifier",
        )
    })?;
    let notification_id = notification_id.ok_or_else(|| {
        windows_notification_payload_error(
            "Windows activation arguments were missing one notification identifier",
        )
    })?;

    Ok(WindowsNotificationActivationPayload {
        source_host_runtime_id,
        notification_id,
        action_id,
        input_id,
    })
}

/// Encode one Windows activation token string as lowercase hex.
fn encode_windows_notification_token_string(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);

    for byte in value.as_bytes() {
        let high = byte >> 4;
        let low = byte & 0x0f;

        encoded.push(hex_digit(high));
        encoded.push(hex_digit(low));
    }

    encoded
}

/// Decode one Windows activation token string from lowercase hex.
fn decode_windows_notification_token_string(value: &str) -> RuntimeResult<String> {
    if value.len() % 2 != 0 {
        return Err(windows_notification_payload_error(
            "Windows activation token had one invalid hex length",
        ));
    }

    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    let mut index = 0;

    while index < bytes.len() {
        let high = decode_hex_digit(bytes[index])?;
        let low = decode_hex_digit(bytes[index + 1])?;
        decoded.push((high << 4) | low);
        index += 2;
    }

    String::from_utf8(decoded).map_err(windows_notification_payload_error)
}

/// Encode one hex nibble as one lowercase hex digit.
fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("invalid hex nibble"),
    }
}

/// Decode one lowercase hex digit into one nibble.
fn decode_hex_digit(value: u8) -> RuntimeResult<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(windows_notification_payload_error(
            "Windows activation token had one invalid hex digit",
        )),
    }
}

/// Read one text-input response from the COM activation callback input list.
pub(super) fn windows_notification_response_text_from_input_data(
    data: *const NOTIFICATION_USER_INPUT_DATA,
    count: u32,
    input_id: Option<&str>,
) -> RuntimeResult<Option<String>> {
    let Some(input_id) = input_id else {
        return Ok(None);
    };

    if data.is_null() || count == 0 {
        return Ok(None);
    }

    let values = unsafe { std::slice::from_raw_parts(data, count as usize) };

    for value in values {
        let key = unsafe { value.Key.to_string() }
            .map_err(super::core::windows_notification_utf16_error)?;

        if key != input_id {
            continue;
        }

        let response = unsafe { value.Value.to_string() }
            .map_err(super::core::windows_notification_utf16_error)?;

        return Ok(Some(response));
    }

    Ok(None)
}

/// Return one stable text-input identifier for one Windows notification action.
pub(super) fn windows_notification_text_input_id(index: usize) -> String {
    format!(
        "{}.{index}",
        super::core::WINDOWS_NOTIFICATION_TEXT_INPUT_ID_PREFIX
    )
}

/// Return one text-input identifier for one action when needed.
pub(super) fn windows_notification_input_id_for_action(
    index: usize,
    action: &NotificationActionValue,
) -> Option<String> {
    if action.style != NotificationActionStyle::TextInput {
        return None;
    }

    Some(windows_notification_text_input_id(index))
}

/// Return the Windows activation type for one action.
fn windows_notification_activation_type(action: &NotificationActionValue) -> &'static str {
    if action.foreground {
        return "foreground";
    }

    "background"
}

/// Return the Windows button style hint for one action when present.
pub(super) fn windows_notification_button_style(
    action: &NotificationActionValue,
) -> Option<&'static str> {
    if action.style == NotificationActionStyle::Destructive {
        return Some("Critical");
    }

    None
}

/// Validate one Windows notification category against supported action semantics.
pub(super) fn validate_windows_notification_category(
    category: &NotificationCategoryValue,
) -> RuntimeResult<()> {
    for action in &category.actions {
        if !action.foreground {
            return Err(not_supported("destack.os.notification.categorySet"));
        }

        if action.authentication_required {
            return Err(not_supported("destack.os.notification.categorySet"));
        }
    }

    Ok(())
}

/// Escape one string for simple notification XML payloads.
pub(super) fn windows_notification_xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{
        validate_windows_notification_category, windows_notification_activation_from_arguments,
        windows_notification_activation_payload, windows_toast_xml,
    };
    use crate::host::{HostSessionId, Platform};
    use crate::platform::os::NotificationActionStyle;
    use crate::platform::os::abi_generated::{
        NotificationActionValue, NotificationCategoryValue, NotificationImmediateTriggerValue,
        NotificationRequestValue, NotificationTriggerValue,
    };
    use crate::platform::os::notification::runtime;

    #[test]
    fn test_windows_toast_xml_includes_actions() {
        let category = NotificationCategoryValue {
            id: "messages".to_string(),
            actions: vec![NotificationActionValue {
                id: "reply".to_string(),
                title: "Reply".to_string(),
                style: NotificationActionStyle::TextInput,
                foreground: true,
                authentication_required: false,
                text_input_button_title: Some("Send".to_string()),
                text_input_placeholder: Some("Reply".to_string()),
            }],
        };

        runtime::set_notification_categories(HostSessionId(7), Platform::Windows, vec![category])
            .unwrap();

        let xml = windows_toast_xml(
            HostSessionId(7),
            "notification-7-1",
            &notification_request(),
        )
        .unwrap();

        assert!(xml.contains("<actions>"));
        assert!(xml.contains("destack.notification.input.0"));
    }

    #[test]
    fn test_windows_activation_payload_roundtrips() {
        let request = notification_request();
        let payload = windows_notification_activation_payload(
            HostSessionId(7),
            "notification-7-1",
            &request,
            Some("reply"),
            Some("destack.notification.input.0"),
        )
        .unwrap();
        let decoded =
            windows_notification_activation_from_arguments(&payload, None, None, None).unwrap();

        assert_eq!(decoded.notification_id, "notification-7-1");
        assert_eq!(decoded.action_id.as_deref(), Some("reply"));
        assert_eq!(
            decoded.input_id.as_deref(),
            Some("destack.notification.input.0")
        );
    }

    #[test]
    fn test_validate_windows_notification_category_rejects_background_actions() {
        let error = validate_windows_notification_category(&NotificationCategoryValue {
            id: "messages".to_string(),
            actions: vec![NotificationActionValue {
                id: "reply".to_string(),
                title: "Reply".to_string(),
                style: NotificationActionStyle::Default,
                foreground: false,
                authentication_required: false,
                text_input_button_title: None,
                text_input_placeholder: None,
            }],
        })
        .expect_err("background Windows actions should be rejected");

        assert!(
            error
                .to_string()
                .contains("destack.os.notification.categorySet")
        );
    }

    fn notification_request() -> NotificationRequestValue {
        NotificationRequestValue {
            title: "title".to_string(),
            subtitle: None,
            body: "body".to_string(),
            tag: "tag".to_string(),
            channel_id: None,
            priority: NotificationPriority::Normal,
            badge_count: None,
            sound: None,
            category_id: Some("messages".to_string()),
            thread_id: None,
            trigger: NotificationTriggerValue::NotificationImmediateTrigger(
                NotificationImmediateTriggerValue {
                    kind: "immediate".to_string(),
                },
            ),
            action_id: None,
        }
    }
}
