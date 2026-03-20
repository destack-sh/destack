use std::process::{Command, Output};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use crate::host::app::background::runtime::desktop_background_test_mode_enabled;

/// Known system launchctl locations on macOS hosts.
const LAUNCHCTL_PATHS: &[&str] = &["/bin/launchctl", "/usr/bin/launchctl"];

/// Return whether launchctl is discoverable on the current host.
pub(super) fn launchd_is_available() -> bool {
    resolved_launchctl_path().is_some()
}

/// Return the active launchd user domain for the current process.
pub(super) fn launchd_domain() -> RuntimeResult<String> {
    let user_id = launchd_user_id()?;

    Ok(format!("gui/{user_id}"))
}

/// Bootstrap one launchd task definition from one plist path.
pub(super) fn bootstrap_background_task(
    operation: &'static str,
    domain: &str,
    plist_path: &str,
) -> RuntimeResult<()> {
    run_launchctl_command(operation, &["bootstrap", domain, plist_path])?;

    Ok(())
}

/// Trigger one launchd task immediately.
pub(super) fn launchd_trigger_background_task(
    operation: &'static str,
    domain: &str,
    label: &str,
) -> RuntimeResult<()> {
    let target = format!("{domain}/{label}");

    run_launchctl_command(operation, &["kickstart", "-k", &target])?;

    Ok(())
}

/// Boot one launchd task out when it is currently registered.
pub(super) fn bootout_background_task_if_present(
    operation: &'static str,
    domain: &str,
    label: &str,
    plist_path: &str,
) -> RuntimeResult<()> {
    if !launchd_service_is_loaded(operation, domain, label)? {
        return Ok(());
    }

    let output = run_launchctl_command_output(operation, &["bootout", domain, plist_path])?;

    require_launchctl_success(operation, output)?;

    Ok(())
}

/// Return whether one launchd service target is currently loaded.
fn launchd_service_is_loaded(
    operation: &'static str,
    domain: &str,
    label: &str,
) -> RuntimeResult<bool> {
    let target = format!("{domain}/{label}");
    let output = run_launchctl_command_output(operation, &["print", &target])?;

    Ok(output.status.success())
}

/// Return the current Unix user id for one launchd bridge.
fn launchd_user_id() -> RuntimeResult<u32> {
    if let Some(value) = std::env::var_os("UID") {
        let value = value.into_string().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "UID",
                "desktop uid must be valid UTF-8",
            ))
            .boxed()
        })?;
        let user_id = value.parse::<u32>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "UID",
                "desktop uid must be one unsigned integer",
            ))
            .boxed()
        })?;

        return Ok(user_id);
    }

    #[cfg(unix)]
    {
        Ok(unsafe { libc::getuid() as u32 })
    }

    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::NotSupported),
            "destack.os.background is not supported on this host",
        ))
        .boxed())
    }
}

/// Run one launchctl command and require process success.
fn run_launchctl_command(operation: &'static str, arguments: &[&str]) -> RuntimeResult<Output> {
    let output = run_launchctl_command_output(operation, arguments)?;

    require_launchctl_success(operation, output)
}

/// Run one launchctl command and return the raw process output.
fn run_launchctl_command_output(
    operation: &'static str,
    arguments: &[&str],
) -> RuntimeResult<Output> {
    let launchctl_path = require_launchctl_path(operation)?;

    if desktop_background_test_mode_enabled() {
        let output = Command::new(launchctl_path)
            .args(arguments)
            .output()
            .map(|mut output| {
                output.status = success_exit_status();
                output.stdout.clear();
                output.stderr.clear();
                output
            })
            .unwrap_or_else(|_| success_output());

        return Ok(output);
    }

    let output = Command::new(launchctl_path)
        .args(arguments)
        .output()
        .map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!("{operation}: failed to run launchctl at {launchctl_path}: {error}"),
            ))
            .boxed()
        })?;

    Ok(output)
}

/// Require one system launchctl path to exist on the current host.
fn require_launchctl_path(operation: &'static str) -> RuntimeResult<&'static str> {
    resolved_launchctl_path().ok_or_else(|| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::NotSupported),
            format!("{operation}: launchctl was not found in a supported system location"),
        ))
        .boxed()
    })
}

/// Return one supported launchctl path when it exists on the current host.
fn resolved_launchctl_path() -> Option<&'static str> {
    LAUNCHCTL_PATHS
        .iter()
        .find(|path| std::path::Path::new(path).is_file())
        .copied()
}

/// Require one launchctl output to have succeeded.
fn require_launchctl_success(operation: &'static str, output: Output) -> RuntimeResult<Output> {
    if output.status.success() {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    let detail = if stderr.is_empty() {
        format!(
            "{operation}: launchctl exited with status {}",
            output.status
        )
    } else {
        format!("{operation}: launchctl failed: {stderr}")
    };

    Err(RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoInvalidData),
        detail,
    ))
    .boxed())
}

/// Return one successful background command output for tests.
fn success_output() -> Output {
    Output {
        status: success_exit_status(),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

/// Return one successful process exit status for tests.
fn success_exit_status() -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        std::process::ExitStatus::from_raw(0)
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;

        std::process::ExitStatus::from_raw(0)
    }

    #[cfg(not(any(unix, windows)))]
    {
        unreachable!("desktop background command tests only run on unix or windows");
    }
}
