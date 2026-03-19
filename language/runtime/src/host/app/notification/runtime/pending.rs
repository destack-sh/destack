use super::posted::{
    notification_identifier_belongs_to_runtime, notification_identifier_for_request,
};
use super::state::{DesktopNotificationRuntimeState, notification_registry};
#[cfg(test)]
use super::state::{desktop_notification_test_mode_enabled, wall_clock_now_ns};
use crate::diagnostic::RuntimeResult;
use crate::host::app::notification::delivery;
#[cfg(test)]
use crate::host::app::notification::time::calendar_date_trigger_unix_ns;
use crate::host::core::HostRequestContext;
use crate::platform::core::io_not_found;
use crate::platform::os::abi_generated::{
    NotificationRequestValue, NotificationScheduledDescriptorValue,
};
#[cfg(test)]
use crate::platform::os::abi_generated::{
    NotificationTimeIntervalTriggerValue, NotificationTriggerValue,
};

/// Return pending scheduled notifications for one runtime.
pub(in crate::host::app::notification) fn list_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        let registry = notification_registry();
        let mut registry = registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_runtime_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));

        let pending_descriptors = runtime_state.pending.values().cloned().collect::<Vec<_>>();

        return Ok(pending_descriptors);
    }

    let pending = delivery::list_native_pending_notifications(context)?;
    let pending = pending
        .into_iter()
        .filter(|descriptor| {
            notification_identifier_belongs_to_runtime(context.host_runtime_id, &descriptor.id)
        })
        .collect();

    Ok(pending)
}

/// Cancel one pending scheduled notification for this runtime.
pub(in crate::host::app::notification) fn cancel_pending_notification(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        let registry = notification_registry();
        let mut registry = registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_runtime_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
        runtime_state.pending.remove(id);

        return Ok(());
    }

    if !notification_identifier_belongs_to_runtime(context.host_runtime_id, id) {
        return Err(io_not_found(
            "destack.os.notification.pendingCancel",
            "notification was not found",
        ));
    }

    delivery::cancel_native_pending_notification(context, id)?;

    let registry = notification_registry();
    let mut registry = registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(context.host_runtime_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
    runtime_state.pending.remove(id);

    Ok(())
}

/// Cancel every pending scheduled notification for this runtime.
pub(in crate::host::app::notification) fn cancel_all_pending_notifications(
    context: &HostRequestContext,
) -> RuntimeResult<()> {
    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        let registry = notification_registry();
        let mut registry = registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_runtime_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
        runtime_state.pending.clear();

        return Ok(());
    }

    let pending_ids = delivery::list_native_pending_notifications(context)?
        .into_iter()
        .filter(|descriptor| {
            notification_identifier_belongs_to_runtime(context.host_runtime_id, &descriptor.id)
        })
        .map(|descriptor| descriptor.id)
        .collect::<Vec<_>>();

    for id in pending_ids {
        delivery::cancel_native_pending_notification(context, &id)?;
    }

    let registry = notification_registry();
    let mut registry = registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(context.host_runtime_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
    runtime_state.pending.clear();

    Ok(())
}

/// Schedule one notification for later delivery.
pub(in crate::host::app::notification) fn schedule_notification(
    context: &HostRequestContext,
    request: NotificationRequestValue,
) -> RuntimeResult<String> {
    #[cfg(test)]
    let scheduled_unix_ns = if desktop_notification_test_mode_enabled() {
        Some(trigger_delivery_unix_ns(&request.trigger)?)
    } else {
        None
    };

    #[cfg(test)]
    if desktop_notification_test_mode_enabled() {
        let registry = notification_registry();
        let mut registry = registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_runtime_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));
        let id =
            notification_identifier_for_request(context.host_runtime_id, runtime_state, &request);

        runtime_state.pending.insert(
            id.clone(),
            NotificationScheduledDescriptorValue {
                id: id.clone(),
                request,
                scheduled_unix_ns,
            },
        );

        return Ok(id);
    }

    let id = {
        let registry = notification_registry();
        let mut registry = registry.lock();
        let runtime_state = registry
            .runtimes
            .entry(context.host_runtime_id)
            .or_insert_with(|| DesktopNotificationRuntimeState::new(context.platform));

        notification_identifier_for_request(context.host_runtime_id, runtime_state, &request)
    };

    delivery::schedule_native_notification(context, &id, &request)?;

    Ok(id)
}

/// Return the wall-clock delivery timestamp for one notification trigger.
#[cfg(test)]
fn trigger_delivery_unix_ns(trigger: &NotificationTriggerValue) -> RuntimeResult<u64> {
    let now_unix_ns = wall_clock_now_ns()?;

    match trigger {
        NotificationTriggerValue::NotificationImmediateTrigger(_) => Ok(now_unix_ns),
        NotificationTriggerValue::NotificationTimeIntervalTrigger(value) => {
            time_interval_trigger_unix_ns(now_unix_ns, value)
        }
        NotificationTriggerValue::NotificationCalendarDateTrigger(value) => {
            calendar_date_trigger_unix_ns(value)
        }
    }
}

/// Return the wall-clock delivery timestamp for one time-interval trigger.
#[cfg(test)]
fn time_interval_trigger_unix_ns(
    now_unix_ns: u64,
    value: &NotificationTimeIntervalTriggerValue,
) -> RuntimeResult<u64> {
    Ok(now_unix_ns.saturating_add(value.interval_ns))
}
