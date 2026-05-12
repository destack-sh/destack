pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
#[cfg(any(unix, windows))]
pub(crate) use std::sync::OnceLock;
pub(crate) use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) use std::time::Duration;

pub(crate) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(crate) use crate::host::operation::{
    background as host_background, location as host_location, notification as host_notification,
};
pub(crate) use crate::host::{
    HostBackgroundEvent, HostEvent, HostEventObserver, HostIntentEvent, HostIntentPayload,
    HostLifecycleState, HostLocationEvent, HostMemoryPressureLevel, HostNotificationEvent,
    HostPowerMode, HostQueue, HostRequestId, HostSessionRegistry,
};
pub(crate) use crate::platform::core::{
    invalid_argument, io_would_block, monotonic_now_ns, not_supported,
};
pub(crate) use crate::platform::diagnostic::PlatformErrorCode;
pub(crate) use crate::platform::os::abi_generated::{
    BackgroundEventOpenOptionsValue, BackgroundEventValue, BackgroundStatusValue,
    BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue, BackgroundTaskResultValue,
    LocationSampleValue, LocationWatchOptionsValue, NotificationCategoryValue,
    NotificationEventOpenOptionsValue, NotificationEventValue, NotificationRequestValue,
    NotificationScheduledDescriptorValue,
};
pub(crate) use crate::platform::os::{
    IntentOpenOptions, LifecycleBackgroundEventValue, LifecycleEventMetadata, LifecycleEventValue,
    LifecycleForegroundEventValue, LifecycleLaunchEventValue, LifecycleLowMemoryEventValue,
    LifecycleLowMemoryPayload, LifecycleLowPowerModeChangedEventValue, LifecycleLowPowerPayload,
    LifecyclePauseEventValue, LifecycleResumeEventValue, LifecycleState,
    LifecycleTerminateEventValue, NetworkEvent, NetworkState, NotificationPermissionState,
    Permission, PermissionState,
};
pub(crate) use crate::platform::resource::{self, ResourceEntry, ResourceKind};
pub(crate) use crate::platform::{PlatformError, fs};
pub(crate) use crate::runtime::{BindingCallContext, RuntimeEventQueue, WorkerCallbackHandle};
pub(crate) use destack_core::{Capture, CaptureMode};
pub(crate) use parking_lot::{Mutex, RwLock};

/// Build one invalid-data runtime error.
pub(crate) fn invalid_data(
    operation: &'static str,
    detail: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        detail,
    ))
    .boxed()
}

/// Encode one owned string path into a native os-path payload.
pub(crate) fn intent_path_from_utf8(binding: &BindingCallContext, value: String) -> fs::OsPath {
    fs::core::os_path_from_utf8_string(binding, value)
}

/// Normalize one host permission token into the public permission selector.
pub(crate) fn parse_host_permission_name(permission: &str) -> Option<Permission> {
    let canonical = permission
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect::<String>();

    match canonical.as_str() {
        "location" => Some(Permission::Location),
        "locationbackground" => Some(Permission::LocationBackground),
        "camera" => Some(Permission::Camera),
        "microphone" => Some(Permission::Microphone),
        "bluetooth" => Some(Permission::Bluetooth),
        "notifications" | "notification" => Some(Permission::Notifications),
        "contactsread" => Some(Permission::ContactsRead),
        "contactswrite" => Some(Permission::ContactsWrite),
        "mediaread" => Some(Permission::MediaRead),
        "mediawrite" => Some(Permission::MediaWrite),
        "motion" => Some(Permission::Motion),
        "clipboardread" => Some(Permission::ClipboardRead),
        "calendarread" => Some(Permission::CalendarRead),
        "calendarwrite" => Some(Permission::CalendarWrite),
        _ => None,
    }
}
