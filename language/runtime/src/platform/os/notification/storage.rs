use std::fs;
use std::path::PathBuf;

use destack_core::fnv1a_64;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    NotificationRequestValue, NotificationScheduledDescriptorValue,
};

/// Prefix for persisted desktop notification scheduler artifacts.
const DESKTOP_NOTIFICATION_SCHEDULER_PREFIX: &str = "destack-notification";
/// Extension for persisted desktop notification records.
const DESKTOP_NOTIFICATION_RECORD_EXTENSION: &str = "record";

/// Persisted scheduled notification record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DesktopScheduledNotificationRecord {
    /// Stable notification identifier.
    pub(super) id: String,
    /// Scheduled notification request payload.
    pub(super) request: NotificationRequestValue,
    /// Earliest scheduled delivery timestamp in UTC nanoseconds when available.
    pub(super) scheduled_unix_ns: Option<u64>,
}

impl DesktopScheduledNotificationRecord {
    /// Return one descriptor view for this scheduled notification record.
    pub(crate) fn descriptor(&self) -> NotificationScheduledDescriptorValue {
        NotificationScheduledDescriptorValue {
            id: self.id.clone(),
            request: self.request.clone(),
            scheduled_unix_ns: self.scheduled_unix_ns,
        }
    }
}

/// Read every persisted scheduled notification record.
pub(crate) fn read_notification_records(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<DesktopScheduledNotificationRecord>> {
    let directory = notification_record_directory(context)?;

    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    let entries = fs::read_dir(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.pendingList: failed to read scheduled notification directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoInvalidData),
                format!(
                    "destack.os.notification.pendingList: invalid scheduled notification entry: {error}"
                ),
            ))
            .boxed()
        })?;
        let path = entry.path();

        if path.extension().and_then(|value| value.to_str())
            != Some(DESKTOP_NOTIFICATION_RECORD_EXTENSION)
        {
            continue;
        }

        let payload = fs::read(&path).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.notification.pendingList: failed to read scheduled notification record {}: {error}",
                    path.display()
                ),
            ))
            .boxed()
        })?;
        let record =
            decode_notification_record("destack.os.notification.pendingList", &path, &payload)?;

        records.push(record);
    }

    Ok(records)
}

/// Read one persisted scheduled notification record when it exists.
pub(crate) fn read_notification_record(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<Option<DesktopScheduledNotificationRecord>> {
    let path = notification_record_path(context, id)?;

    if !path.exists() {
        return Ok(None);
    }

    let payload = fs::read(&path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.pendingList: failed to read scheduled notification record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;
    let record =
        decode_notification_record("destack.os.notification.pendingList", &path, &payload)?;

    Ok(Some(record))
}

/// Persist one scheduled notification record.
pub(crate) fn write_notification_record(
    context: &HostRequestContext,
    record: &DesktopScheduledNotificationRecord,
) -> RuntimeResult<()> {
    let directory = ensure_notification_record_directory(context)?;
    let path = notification_record_path(context, &record.id)?;
    let payload = encode_notification_record("destack.os.notification.schedule", record)?;

    fs::create_dir_all(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.schedule: failed to create scheduled notification directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    fs::write(&path, payload).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.schedule: failed to write scheduled notification record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    Ok(())
}

/// Decode one persisted scheduled notification record.
fn decode_notification_record(
    operation: &'static str,
    path: &std::path::Path,
    payload: &[u8],
) -> RuntimeResult<DesktopScheduledNotificationRecord> {
    from_bytes::<DesktopScheduledNotificationRecord>(payload).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "{operation}: invalid scheduled notification record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })
}

/// Encode one persisted scheduled notification record.
fn encode_notification_record(
    operation: &'static str,
    record: &DesktopScheduledNotificationRecord,
) -> RuntimeResult<Vec<u8>> {
    to_allocvec(record).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            format!("{operation}: failed to encode scheduled notification record: {error}"),
        ))
        .boxed()
    })
}

/// Remove one persisted scheduled notification record when it exists.
pub(crate) fn remove_notification_record(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<()> {
    let path = notification_record_path(context, id)?;

    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.pendingCancel: failed to remove scheduled notification record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    Ok(())
}

/// Ensure the persisted scheduled notification directory exists.
pub(crate) fn ensure_notification_record_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let directory = notification_record_directory(context)?;

    fs::create_dir_all(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.schedule: failed to create scheduled notification directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    Ok(directory)
}

/// Return the persisted scheduler artifact directory for one runtime.
pub(crate) fn notification_scheduler_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let root = if let Some(state_directory) = &context.os_options.state_directory {
        state_directory.join("notification")
    } else {
        default_notification_state_directory(context.platform)?
    };

    Ok(root.join("scheduler"))
}

/// Ensure the persisted scheduler artifact directory exists.
pub(crate) fn ensure_notification_scheduler_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let directory = notification_scheduler_directory(context)?;

    fs::create_dir_all(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.schedule: failed to create notification scheduler directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    Ok(directory)
}

/// Return one stable scheduler key for one notification identifier.
pub(crate) fn notification_scheduler_key(id: &str) -> String {
    let hash = fnv1a_64(id.as_bytes());

    format!("{DESKTOP_NOTIFICATION_SCHEDULER_PREFIX}-{hash:016x}")
}

/// Return one wrapper script path for one scheduled notification identifier.
pub(crate) fn notification_wrapper_script_path(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<PathBuf> {
    let directory = notification_scheduler_directory(context)?;
    let key = notification_scheduler_key(id);

    Ok(directory.join(format!("{key}.sh")))
}

/// Return the systemd service unit path for one scheduled notification identifier.
#[cfg(target_os = "linux")]
pub(crate) fn notification_systemd_service_path(id: &str) -> RuntimeResult<PathBuf> {
    let directory = notification_systemd_user_directory()?;
    let key = notification_scheduler_key(id);

    Ok(directory.join(format!("{key}.service")))
}

/// Return the systemd timer unit path for one scheduled notification identifier.
#[cfg(target_os = "linux")]
pub(crate) fn notification_systemd_timer_path(id: &str) -> RuntimeResult<PathBuf> {
    let directory = notification_systemd_user_directory()?;
    let key = notification_scheduler_key(id);

    Ok(directory.join(format!("{key}.timer")))
}

/// Return the persisted scheduled notification record path for one identifier.
fn notification_record_path(context: &HostRequestContext, id: &str) -> RuntimeResult<PathBuf> {
    let directory = notification_record_directory(context)?;
    let hash = fnv1a_64(id.as_bytes());

    Ok(directory.join(format!(
        "{hash:016x}.{DESKTOP_NOTIFICATION_RECORD_EXTENSION}"
    )))
}

/// Return the persisted scheduled notification record directory.
fn notification_record_directory(context: &HostRequestContext) -> RuntimeResult<PathBuf> {
    let root = if let Some(state_directory) = &context.os_options.state_directory {
        state_directory.join("notification")
    } else {
        default_notification_state_directory(context.platform)?
    };

    Ok(root.join("scheduled"))
}

/// Return the default persisted notification state directory for one desktop host.
fn default_notification_state_directory(platform: Platform) -> RuntimeResult<PathBuf> {
    match platform {
        Platform::MacOS => {
            let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
                return Err(not_supported("destack.os.notification"));
            };

            Ok(home
                .join("Library")
                .join("Application Support")
                .join("destack")
                .join("notification"))
        }
        Platform::Windows => {
            let Some(root) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
                return Err(not_supported("destack.os.notification"));
            };

            Ok(root.join("destack").join("notification"))
        }
        Platform::Linux => {
            let root = if let Some(state_home) = std::env::var_os("XDG_STATE_HOME") {
                PathBuf::from(state_home)
            } else {
                let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
                    return Err(not_supported("destack.os.notification"));
                };

                home.join(".local").join("state")
            };

            Ok(root.join("destack").join("notification"))
        }
        _ => Err(not_supported("destack.os.notification")),
    }
}

/// Return the systemd user unit directory for desktop notifications.
#[cfg(target_os = "linux")]
fn notification_systemd_user_directory() -> RuntimeResult<PathBuf> {
    let directory = if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(config_home)
    } else {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return Err(not_supported("destack.os.notification.schedule"));
        };

        home.join(".config")
    };

    Ok(directory.join("systemd").join("user"))
}
