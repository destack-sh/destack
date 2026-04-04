use crate::diagnostic::RuntimeResult;
use crate::host::{HostSessionId, RequestContext};
use crate::platform::core::io_not_found;
use crate::platform::os::abi_generated::NotificationRequestValue;
use crate::platform::os::notification::{delivery, runtime};

use super::state::{DesktopNotificationRuntimeState, notification_runtime_service};

/// Cancel one posted notification for this runtime.
pub(crate) fn cancel_notification(context: &RequestContext, id: &str) -> RuntimeResult<()> {
    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(context.host_session_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));

    if !runtime_state.posted.contains_key(id) {
        return Err(io_not_found(
            "destack.os.notification.cancel",
            "notification was not found",
        ));
    }

    drop(registry);

    delivery::cancel_native_notification(context, id)?;

    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(context.host_session_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
    runtime_state.posted.remove(id);

    Ok(())
}

/// Cancel every posted notification for this runtime.
pub(crate) fn cancel_all_notifications(context: &RequestContext) -> RuntimeResult<()> {
    let posted_ids = {
        let service = notification_runtime_service();
        let mut registry = service.registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_session_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
        runtime_state.posted.keys().cloned().collect::<Vec<_>>()
    };

    for id in posted_ids {
        delivery::cancel_native_notification(context, &id)?;

        let service = notification_runtime_service();
        let mut registry = service.registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_session_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
        runtime_state.posted.remove(&id);
    }

    Ok(())
}

/// Remove one posted notification from runtime state when present.
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn remove_posted_notification(host_session_id: HostSessionId, id: &str) {
    let service = notification_runtime_service();
    let mut registry = service.registry.lock();

    if let Some(runtime_state) = registry.runtimes.get_mut(&host_session_id) {
        runtime_state.posted.remove(id);
    }
}

/// Post one notification immediately and emit one delivered event.
pub(crate) fn post_notification(
    context: &RequestContext,
    request: NotificationRequestValue,
) -> RuntimeResult<String> {
    let (id, sequence) = {
        let service = notification_runtime_service();
        let mut registry = service.registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_session_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
        let id =
            notification_identifier_for_request(context.host_session_id, runtime_state, &request);
        let sequence = runtime_state.next_sequence();

        (id, sequence)
    };

    delivery::deliver_notification(context, &id, &request)?;

    // only record posted state after the host accepted delivery
    {
        let service = notification_runtime_service();
        let mut registry = service.registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_session_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));

        runtime_state.posted.insert(id.clone(), request.clone());
    }

    runtime::publish_delivered_notification(
        context.host_session_id,
        context.platform,
        id.clone(),
        request,
        sequence,
    )?;

    Ok(id)
}

/// Return the stable identifier for one posted or scheduled request.
pub(crate) fn notification_identifier_for_request(
    host_session_id: HostSessionId,
    runtime_state: &mut DesktopNotificationRuntimeState,
    request: &NotificationRequestValue,
) -> String {
    if !request.tag.is_empty() {
        return format!("notification-{}-{}", host_session_id.0, request.tag);
    }

    let id = format!(
        "notification-{}-{}",
        host_session_id.0, runtime_state.next_identifier
    );
    runtime_state.next_identifier = runtime_state.next_identifier.wrapping_add(1);

    id
}

/// Return whether one host notification identifier belongs to one runtime.
pub(crate) fn notification_identifier_belongs_to_runtime(
    host_session_id: HostSessionId,
    id: &str,
) -> bool {
    let expected_prefix = format!("notification-{}-", host_session_id.0);

    id.starts_with(&expected_prefix)
}

#[cfg(test)]
mod tests {
    use super::{notification_identifier_belongs_to_runtime, notification_identifier_for_request};
    use crate::host::{HostSessionId, Platform};
    use crate::platform::os::NotificationPriority;
    use crate::platform::os::abi_generated::{
        NotificationImmediateTriggerValue, NotificationRequestValue, NotificationTriggerValue,
    };
    use crate::platform::os::notification::runtime::state::DesktopNotificationRuntimeState;

    /// Preserve stable replacement ids for tagged notification requests.
    #[test]
    fn test_notification_identifier_reuses_tagged_request_id() {
        let mut runtime_state = DesktopNotificationRuntimeState::new(Platform::Linux);
        let request = notification_request("stable-tag");
        let first_id =
            notification_identifier_for_request(HostSessionId(7), &mut runtime_state, &request);
        runtime_state
            .posted
            .insert(first_id.clone(), request.clone());
        let second_id =
            notification_identifier_for_request(HostSessionId(7), &mut runtime_state, &request);

        assert_eq!(first_id, second_id);
        assert_eq!(first_id, "notification-7-stable-tag");
    }

    /// Keep generated notification identifiers unique across runtimes.
    #[test]
    fn test_notification_identifier_namespaces_runtime_generated_ids() {
        let request = notification_request("");
        let mut first_runtime = DesktopNotificationRuntimeState::new(Platform::Linux);
        let mut second_runtime = DesktopNotificationRuntimeState::new(Platform::Linux);
        let first_id =
            notification_identifier_for_request(HostSessionId(7), &mut first_runtime, &request);
        let second_id =
            notification_identifier_for_request(HostSessionId(9), &mut second_runtime, &request);

        assert_eq!(first_id, "notification-7-1");
        assert_eq!(second_id, "notification-9-1");
    }

    /// Match runtime ownership against the host notification id prefix.
    #[test]
    fn test_notification_identifier_belongs_to_runtime_matches_prefixed_ids() {
        assert!(notification_identifier_belongs_to_runtime(
            HostSessionId(7),
            "notification-7-stable-tag"
        ));
        assert!(notification_identifier_belongs_to_runtime(
            HostSessionId(7),
            "notification-7-1"
        ));
        assert!(!notification_identifier_belongs_to_runtime(
            HostSessionId(7),
            "notification-9-1"
        ));
        assert!(!notification_identifier_belongs_to_runtime(
            HostSessionId(7),
            "stable-tag"
        ));
    }

    /// Build one minimal notification request for tests.
    fn notification_request(tag: &str) -> NotificationRequestValue {
        NotificationRequestValue {
            title: "title".to_string(),
            subtitle: None,
            body: "body".to_string(),
            tag: tag.to_string(),
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
            action_id: None,
        }
    }
}
