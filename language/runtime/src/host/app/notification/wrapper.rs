use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use super::linux::DESKTOP_NOTIFICATION_IDENTIFIER_ENV;
use super::storage::{ensure_notification_scheduler_directory, notification_wrapper_script_path};

/// Write one persisted wrapper script for one scheduled notification.
pub(super) fn write_notification_wrapper_script(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<PathBuf> {
    let path = notification_wrapper_script_path(context, id)?;
    let payload = render_notification_wrapper(id)?;

    ensure_notification_scheduler_directory(context)?;
    fs::write(&path, payload).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.schedule: failed to write notification wrapper {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    let permissions = fs::Permissions::from_mode(0o755);

    fs::set_permissions(&path, permissions).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.notification.schedule: failed to chmod notification wrapper {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    Ok(path)
}

/// Render one posix wrapper script for the current runtime invocation.
fn render_notification_wrapper(id: &str) -> RuntimeResult<String> {
    let (executable, arguments) = current_invocation_utf8("destack.os.notification.schedule")?;
    let executable = shell_quote(&executable);
    let arguments = arguments
        .iter()
        .map(|argument| shell_quote(argument))
        .collect::<Vec<_>>()
        .join(" ");
    let id = shell_quote(id);
    let exec_line = if arguments.is_empty() {
        format!("exec {executable}")
    } else {
        format!("exec {executable} {arguments}")
    };

    Ok(format!(
        "#!/bin/sh\nexport {DESKTOP_NOTIFICATION_IDENTIFIER_ENV}={id}\n{exec_line}\n"
    ))
}

/// Return the current runtime invocation as utf-8 executable and argument strings.
fn current_invocation_utf8(operation: &'static str) -> RuntimeResult<(String, Vec<String>)> {
    let executable = std::env::current_exe().map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            format!("{operation}: current executable unavailable: {error}"),
        ))
        .boxed()
    })?;
    let executable = executable.into_os_string().into_string().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "notification executable path must be valid UTF-8",
        ))
        .boxed()
    })?;
    let mut arguments = Vec::new();

    for argument in std::env::args_os().skip(1) {
        let argument = argument.into_string().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "arguments",
                "notification invocation arguments must be valid UTF-8",
            ))
            .boxed()
        })?;
        arguments.push(argument);
    }

    Ok((executable, arguments))
}

/// Return one shell-safe single-quoted argument.
fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let escaped = value.replace('\'', "'\"'\"'");

    format!("'{escaped}'")
}

/// Return one systemd-safe quoted argument.
pub(super) fn systemd_quote_argument(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");

    format!("\"{escaped}\"")
}
