use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::host::core::HostSessionId;
use crate::platform::os::abi_generated::NotificationCategoryValue;
use crate::platform::os::notification::delivery;

use super::state::{DesktopNotificationRuntimeState, notification_runtime_service};

/// Return registered notification categories for one runtime.
pub(crate) fn list_notification_categories(
    host_session_id: HostSessionId,
    platform: Platform,
) -> RuntimeResult<Vec<NotificationCategoryValue>> {
    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(host_session_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(platform));

    Ok(runtime_state.categories.clone())
}

/// Read one registered notification category for one runtime when present.
#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
pub(crate) fn notification_category(
    host_session_id: HostSessionId,
    category_id: &str,
) -> RuntimeResult<Option<NotificationCategoryValue>> {
    let service = notification_runtime_service();
    let registry = service.registry.lock();
    let category = registry
        .runtimes
        .get(&host_session_id)
        .and_then(|runtime_state| {
            runtime_state
                .categories
                .iter()
                .find(|category| category.id == category_id)
        })
        .cloned();

    Ok(category)
}

/// Replace registered notification categories for one runtime.
pub(crate) fn set_notification_categories(
    host_session_id: HostSessionId,
    platform: Platform,
    categories: Vec<NotificationCategoryValue>,
) -> RuntimeResult<()> {
    delivery::set_native_notification_categories(platform, &categories)?;

    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(host_session_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(platform));
    runtime_state.categories = categories;

    Ok(())
}
