use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;

use crate::platform::os::background::runtime::DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV;
use crate::platform::os::background::storage::{
    background_wrapper_script_path, ensure_background_scheduler_directory,
};

/// Write one persisted background wrapper script for the current runtime invocation.
pub(crate) fn write_background_wrapper_script(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<PathBuf> {
    let path = background_wrapper_script_path(context, identifier)?;
    let payload = match context.platform {
        Platform::Windows => render_windows_background_wrapper(identifier)?,
        Platform::MacOS | Platform::Linux => render_posix_background_wrapper(identifier)?,
        _ => return Err(not_supported("destack.os.background.register")),
    };

    ensure_background_scheduler_directory(context)?;
    fs::write(&path, payload).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background.register: failed to write background wrapper {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o755);

        fs::set_permissions(&path, permissions).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.register: failed to chmod background wrapper {}: {error}",
                    path.display()
                ),
            ))
            .boxed()
        })?;
    }

    Ok(path)
}

/// Render one posix wrapper script for the current runtime invocation.
pub(super) fn render_posix_background_wrapper(identifier: &str) -> RuntimeResult<String> {
    let (executable, arguments) = current_invocation_utf8("destack.os.background.register")?;
    let executable = shell_quote(&executable);
    let arguments = arguments
        .iter()
        .map(|argument| shell_quote(argument))
        .collect::<Vec<_>>()
        .join(" ");
    let identifier = shell_quote(identifier);
    let exec_line = if arguments.is_empty() {
        format!("exec {executable}")
    } else {
        format!("exec {executable} {arguments}")
    };

    Ok(format!(
        "#!/bin/sh\nexport {DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV}={identifier}\n{exec_line}\n"
    ))
}

/// Render one Windows wrapper script for the current runtime invocation.
pub(super) fn render_windows_background_wrapper(identifier: &str) -> RuntimeResult<String> {
    if identifier.contains('"') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "identifier",
            "windows background identifier must not contain double quotes",
        ))
        .boxed());
    }

    let (executable, arguments) = current_invocation_utf8("destack.os.background.register")?;
    let mut command_line = cmd_quote_argument(&executable);

    for argument in arguments {
        command_line.push(' ');
        command_line.push_str(&cmd_quote_argument(&argument));
    }

    Ok(format!(
        "@echo off\r\nsetlocal\r\nset \"{DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV}={identifier}\"\r\n{command_line}\r\n"
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
            "background executable path must be valid UTF-8",
        ))
        .boxed()
    })?;
    let mut arguments = Vec::new();

    for argument in std::env::args_os().skip(1) {
        let argument = argument.into_string().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "arguments",
                "background invocation arguments must be valid UTF-8",
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

/// Return one Windows command-line quoted argument.
fn cmd_quote_argument(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }

    let needs_quotes = value
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || matches!(byte, b'"'));

    if !needs_quotes {
        return value.to_string();
    }

    let mut quoted = String::with_capacity(value.len() + 2);
    let mut backslash_count = 0usize;
    quoted.push('"');

    for character in value.chars() {
        if character == '\\' {
            backslash_count += 1;
            continue;
        }

        if character == '"' {
            quoted.push_str(&"\\".repeat(backslash_count * 2 + 1));
            quoted.push('"');
            backslash_count = 0;
            continue;
        }

        if backslash_count > 0 {
            quoted.push_str(&"\\".repeat(backslash_count));
            backslash_count = 0;
        }

        quoted.push(character);
    }

    if backslash_count > 0 {
        quoted.push_str(&"\\".repeat(backslash_count * 2));
    }

    quoted.push('"');

    quoted
}

/// Return one systemd-safe quoted argument.
#[cfg(target_os = "linux")]
pub(crate) fn systemd_quote_argument(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");

    format!("\"{escaped}\"")
}
