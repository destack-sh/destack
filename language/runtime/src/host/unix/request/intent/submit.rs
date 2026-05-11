use std::ffi::OsString;
use std::process::Command;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::core::{invalid_argument, io_operation_error, not_supported};
use crate::platform::diagnostic::PlatformError;
use crate::platform::fs;
use crate::platform::fs::core as core_fs;
use crate::runtime::action::{HostAction, HostActionSet};

/// Canonical operation name for `intentOpenPath`.
const HOST_INTENT_OPEN_PATH_OPERATION: &str = "destack.os.intent.openPath";
/// Canonical operation name for `intentOpenUrl`.
const HOST_INTENT_OPEN_URL_OPERATION: &str = "destack.os.intent.openUrl";

/// Return request actions for one Unix desktop request adapter.
pub(crate) fn request_actions() -> HostActionSet {
    let mut actions = HostActionSet::new();

    // desktop launchers expose outbound intent routing when present
    if unix_launcher().is_some() {
        actions.insert_action(HostAction::OsIntentWrite);
    }

    actions
}

/// Submit one normalized Unix desktop host request.
pub(crate) fn submit_request(request: HostRequest) -> RuntimeResult<HostRequestOutcome> {
    match request {
        HostRequest::OsIntentCanOpenUrl { .. } => Ok(HostRequestOutcome::immediate(
            HostRequestResult::Bool(unix_launcher().is_some()),
        )),
        HostRequest::OsIntentOpenUrl { url } => {
            open_url_target(&url)?;

            Ok(HostRequestOutcome::immediate(HostRequestResult::None))
        }
        HostRequest::OsIntentOpenPath { path } => {
            open_path_target(path)?;

            Ok(HostRequestOutcome::immediate(HostRequestResult::None))
        }
        _ => Err(not_supported(request.operation_name())),
    }
}

/// Route one URL open request through the desktop Unix host.
fn open_url_target(url: &str) -> RuntimeResult<()> {
    unix_open_target(url, HOST_INTENT_OPEN_URL_OPERATION)
}

/// Route one path open request through the desktop Unix host.
fn open_path_target(path: fs::OsPath) -> RuntimeResult<()> {
    let path = core_fs::os_path_to_utf8_string(path, "path")?;

    if path.is_empty() {
        return Err(invalid_argument("path", "path must not be empty"));
    }

    unix_open_path(OsString::from(path), HOST_INTENT_OPEN_PATH_OPERATION)
}

/// Open one target string with the available Unix desktop launcher.
fn unix_open_target(target: &str, operation: &'static str) -> RuntimeResult<()> {
    let Some(launcher) = unix_launcher() else {
        return Err(not_supported(operation));
    };

    let status = Command::new(launcher.command)
        .args(launcher.arguments)
        .arg(target)
        .status()
        .map_err(|error| {
            RuntimeError::from(PlatformError::io(format!(
                "{operation}: launcher execution failed: {error}"
            )))
            .boxed()
        })?;

    if status.success() {
        return Ok(());
    }

    Err(io_operation_error(
        operation,
        None,
        format!("launcher exited with status {status}"),
    ))
}

/// Open one filesystem path with the available Unix desktop launcher.
fn unix_open_path(path: OsString, operation: &'static str) -> RuntimeResult<()> {
    let Some(launcher) = unix_launcher() else {
        return Err(not_supported(operation));
    };

    let status = Command::new(launcher.command)
        .args(launcher.arguments)
        .arg(path)
        .status()
        .map_err(|error| {
            RuntimeError::from(PlatformError::io(format!(
                "{operation}: launcher execution failed: {error}"
            )))
            .boxed()
        })?;

    if status.success() {
        return Ok(());
    }

    Err(io_operation_error(
        operation,
        None,
        format!("launcher exited with status {status}"),
    ))
}

/// One Unix desktop launcher candidate.
struct UnixLauncher {
    /// Executable path.
    command: &'static str,
    /// Static launcher arguments.
    arguments: &'static [&'static str],
}

/// Return the first available Unix desktop launcher.
fn unix_launcher() -> Option<UnixLauncher> {
    let candidates = [
        UnixLauncher {
            command: "/usr/bin/open",
            arguments: &[],
        },
        UnixLauncher {
            command: "/usr/bin/xdg-open",
            arguments: &[],
        },
        UnixLauncher {
            command: "/usr/bin/gio",
            arguments: &["open"],
        },
    ];

    candidates
        .into_iter()
        .find(|launcher| std::path::Path::new(launcher.command).exists())
}
