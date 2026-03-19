use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::host::app::notification::delivery;
use crate::host::core::HostRuntimeId;
use crate::platform::os::abi_generated::NotificationCategoryValue;

use super::state::{DesktopNotificationRuntimeState, notification_registry};

/// Return registered notification categories for one runtime.
pub(in crate::host::app::notification) fn list_notification_categories(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
) -> RuntimeResult<Vec<NotificationCategoryValue>> {
    let registry = notification_registry();
    let mut registry = registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(host_runtime_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(platform));

    Ok(runtime_state.categories.clone())
}

/// Read one registered notification category for one runtime when present.
#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
pub(in crate::host::app::notification) fn notification_category(
    host_runtime_id: HostRuntimeId,
    category_id: &str,
) -> RuntimeResult<Option<NotificationCategoryValue>> {
    let registry = notification_registry();
    let registry = registry.lock();
    let category = registry
        .runtimes
        .get(&host_runtime_id)
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
pub(in crate::host::app::notification) fn set_notification_categories(
    host_runtime_id: HostRuntimeId,
    platform: Platform,
    categories: Vec<NotificationCategoryValue>,
) -> RuntimeResult<()> {
    delivery::set_native_notification_categories(platform, &categories)?;

    let registry = notification_registry();
    let mut registry = registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(host_runtime_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(platform));
    runtime_state.categories = categories;

    Ok(())
}
