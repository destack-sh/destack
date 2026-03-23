use std::fs;
use std::path::PathBuf;

use destack_core::fnv1a_64;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::HostRequestContext;
#[cfg(target_os = "macos")]
use crate::host::macos::identity::resolved_application_identifier;
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;

use super::core::DESKTOP_BACKGROUND_SCHEDULER_PREFIX;

/// Return the persisted scheduler artifact directory for one runtime.
pub(crate) fn background_scheduler_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let root = background_state_directory(context)?;

    Ok(root.join("scheduler"))
}

/// Ensure the persisted scheduler artifact directory exists.
pub(crate) fn ensure_background_scheduler_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let directory = background_scheduler_directory(context)?;

    fs::create_dir_all(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background.register: failed to create background scheduler directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    Ok(directory)
}

/// Return one stable scheduler key for one background task identifier.
pub(crate) fn background_scheduler_key(identifier: &str) -> String {
    let hash = fnv1a_64(identifier.as_bytes());

    format!("{DESKTOP_BACKGROUND_SCHEDULER_PREFIX}-{hash:016x}")
}

/// Return one wrapper script path for one background task identifier.
pub(crate) fn background_wrapper_script_path(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<PathBuf> {
    let directory = background_scheduler_directory(context)?;
    let key = background_scheduler_key(identifier);
    let extension = if matches!(context.platform, Platform::Windows) {
        "cmd"
    } else {
        "sh"
    };

    Ok(directory.join(format!("{key}.{extension}")))
}

/// Return one launchd label for one background task identifier.
pub(crate) fn background_launchd_label(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<String> {
    let app_identifier = resolved_application_identifier(context)?;
    let hash = fnv1a_64(identifier.as_bytes());

    Ok(format!("{app_identifier}.background.{hash:016x}"))
}

/// Return the launchd plist path for one background task identifier.
pub(crate) fn background_launchd_plist_path(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<PathBuf> {
    let label = background_launchd_label(context, identifier)?;
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Err(not_supported("destack.os.background.register"));
    };

    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist")))
}

/// Return the systemd user unit directory for desktop background tasks.
#[cfg(target_os = "linux")]
pub(crate) fn background_systemd_service_path(identifier: &str) -> RuntimeResult<PathBuf> {
    let directory = background_systemd_user_directory()?;
    let key = background_scheduler_key(identifier);

    Ok(directory.join(format!("{key}.service")))
}

/// Return the systemd timer path for one background task identifier.
#[cfg(target_os = "linux")]
pub(crate) fn background_systemd_timer_path(identifier: &str) -> RuntimeResult<PathBuf> {
    let directory = background_systemd_user_directory()?;
    let key = background_scheduler_key(identifier);

    Ok(directory.join(format!("{key}.timer")))
}

/// Return the persisted background task directory for one runtime.
pub(crate) fn background_task_directory(context: &HostRequestContext) -> RuntimeResult<PathBuf> {
    let root = background_state_directory(context)?;

    Ok(root.join("tasks"))
}

/// Return the persisted record path for one background task identifier.
pub(crate) fn background_task_record_path(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<PathBuf> {
    let directory = background_task_directory(context)?;
    let hash = fnv1a_64(identifier.as_bytes());

    Ok(directory.join(format!("{hash:016x}.json")))
}

/// Return the effective state root for one runtime.
fn background_state_directory(context: &HostRequestContext) -> RuntimeResult<PathBuf> {
    let root = if let Some(state_directory) = &context.os_options.state_directory {
        state_directory.join("background")
    } else {
        default_background_state_directory(context.platform)?
    };

    Ok(root)
}

/// Return the default persisted background state directory for one desktop host.
fn default_background_state_directory(platform: Platform) -> RuntimeResult<PathBuf> {
    match platform {
        Platform::MacOS => {
            let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
                return Err(not_supported("destack.os.background"));
            };

            Ok(home
                .join("Library")
                .join("Application Support")
                .join("destack")
                .join("background"))
        }
        Platform::Windows => {
            let Some(root) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
                return Err(not_supported("destack.os.background"));
            };

            Ok(root.join("destack").join("background"))
        }
        Platform::Linux => {
            let root = if let Some(state_home) = std::env::var_os("XDG_STATE_HOME") {
                PathBuf::from(state_home)
            } else {
                let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
                    return Err(not_supported("destack.os.background"));
                };

                home.join(".local").join("state")
            };

            Ok(root.join("destack").join("background"))
        }
        _ => Err(not_supported("destack.os.background")),
    }
}

/// Return the systemd user unit directory for desktop background tasks.
#[cfg(target_os = "linux")]
fn background_systemd_user_directory() -> RuntimeResult<PathBuf> {
    let directory = if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(config_home)
    } else {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return Err(not_supported("destack.os.background.register"));
        };

        home.join(".config")
    };

    Ok(directory.join("systemd").join("user"))
}
