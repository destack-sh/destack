#[cfg(unix)]
use std::os::fd::AsRawFd;

use super::{
    ProcessFdActionSpec, ProcessSpawnOptionsSpec, ProcessStdioSpec, shell_exit_command,
    shell_sleep_then_exit_command, spawn_shell, with_harness_context,
};
#[cfg(unix)]
use super::{assert_platform_error_code, assert_platform_error_codes, is_would_block};
#[cfg(unix)]
use super::{fork_child_sleep_then_exit, unique_temp_file_path};
#[cfg(unix)]
use crate::diagnostic::RuntimeError;
#[cfg(unix)]
use crate::platform::PlatformError;
#[cfg(unix)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::fs;
#[cfg(unix)]
use crate::platform::process::ProcessFdActionKind;
#[cfg(unix)]
use crate::platform::process::ProcessId;
use crate::platform::process::{
    ProcessFdFlags, ProcessStdioKind, ProcessWaitFlags, ProcessWaitKind,
};
#[cfg(unix)]
use crate::platform::process::{ProcessFdSignalFlags, Signal};
#[cfg(unix)]
use crate::platform::resource::{ProcessFdHandle, ResourceId};

/// Read the current process working directory as UTF-8 text.
fn current_working_directory() -> String {
    std::env::current_dir()
        .expect("cwd should succeed")
        .to_string_lossy()
        .to_string()
}

/// Spawn a child process, wait for exit, and reject stale handle reuse.
#[cfg(unix)]
#[test]
fn test_process_spawn_wait_handle_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let command = "/bin/sh";
        let arguments = vec!["-c".to_string(), "exit 17".to_string()];
        let environment = Vec::new();

        // spawn should exit with the requested status
        let handle = context.spawn(command, &arguments, &environment, &options)?;
        let status = context.wait(handle, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 17);

        // consumed handles should not remain waitable
        assert_platform_error_codes(
            context.try_wait(handle),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::InvalidArgumentValue,
            ],
        )?;

        Ok(())
    });
}

/// Wait on a process-fd handle for a forked child exit.
#[cfg(unix)]
#[test]
fn test_process_fd_wait_roundtrip() {
    with_harness_context(|mut context| {
        let child_pid = fork_child_sleep_then_exit(1, 31)?;
        let handle = context.process_fd_open(child_pid, ProcessFdFlags(0))?;
        let status = context.process_fd_wait(handle, 2_000_000_000)?;

        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 31);

        context.process_fd_close(handle)?;
        Ok(())
    });
}

/// Reject forged process-fd handles created from process handles.
#[cfg(unix)]
#[test]
fn test_process_fd_close_rejects_forged_process_handle() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let command = "/bin/sh";
        let arguments = vec!["-c".to_string(), "sleep 1; exit 0".to_string()];
        let environment = Vec::new();

        // forged process-fd handles should be rejected
        let process_handle = context.spawn(command, &arguments, &environment, &options)?;
        let forged_handle = ProcessFdHandle(process_handle.0);
        assert_platform_error_code(
            context.process_fd_close(forged_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let status = context.wait(process_handle, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);

        Ok(())
    });
}

/// Spawn with explicit stdio/actions and observe exit status.
#[cfg(any(unix, windows))]
#[test]
fn test_process_spawn_with_actions_wait_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let (command, arguments) = shell_exit_command(23);
        let options = ProcessSpawnOptionsSpec {
            cwd: cwd.clone(),
            detached: false,
            reset_signals: true,
            new_process_group: false,
        };
        let environment = Vec::new();
        let stdio = vec![
            ProcessStdioSpec {
                kind: ProcessStdioKind::Null,
                descriptor: 0,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Null,
                descriptor: 0,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Null,
                descriptor: 0,
            },
        ];
        let actions = Vec::<ProcessFdActionSpec>::new();

        let handle = context.spawn_with_actions(
            &command,
            &arguments,
            &environment,
            &options,
            &stdio,
            &actions,
        )?;
        let status = context.wait(handle, ProcessWaitFlags(0))?;

        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 23);

        Ok(())
    });
}

/// Wait a short-lived spawned process after a delay and still observe exit status.
#[cfg(any(unix, windows))]
#[test]
fn test_process_spawn_wait_after_exit_delay_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let (command, arguments) = shell_exit_command(29);
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let environment = Vec::new();
        let stdio = Vec::<ProcessStdioSpec>::new();
        let actions = Vec::<ProcessFdActionSpec>::new();

        let handle = context.spawn_with_actions(
            &command,
            &arguments,
            &environment,
            &options,
            &stdio,
            &actions,
        )?;

        // allow the child to exit before the wait call
        std::thread::sleep(std::time::Duration::from_millis(100));

        // waits should still observe the terminal status
        let status = context.wait(handle, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 29);

        Ok(())
    });
}

/// Redirect stdout through an open fd action and verify captured output.
#[cfg(unix)]
#[test]
fn test_process_spawn_with_actions_open_stdout_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let output_path = unique_temp_file_path("SPAWN_OPEN_STDOUT");
        let output_path_text = output_path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&output_path);

        let options = ProcessSpawnOptionsSpec {
            cwd: cwd.clone(),
            detached: false,
            reset_signals: true,
            new_process_group: false,
        };
        let command = "/bin/sh".to_string();
        let arguments = vec!["-c".to_string(), "printf open-action".to_string()];
        let environment = Vec::new();
        let stdio = Vec::<ProcessStdioSpec>::new();
        let actions = vec![ProcessFdActionSpec {
            op: ProcessFdActionKind::Open,
            source: 0,
            target: 1,
            path: output_path_text.clone(),
            flags: fs::OpenFlags((libc::O_CREAT | libc::O_TRUNC | libc::O_WRONLY) as u32),
            mode: fs::FileMode(0o644),
        }];

        let handle = context.spawn_with_actions(
            &command,
            &arguments,
            &environment,
            &options,
            &stdio,
            &actions,
        )?;
        let status = context.wait(handle, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 0);

        // output file should contain child stdout payload
        let output = std::fs::read_to_string(&output_path).expect("output file should be readable");
        assert_eq!(output, "open-action");
        let _ = std::fs::remove_file(&output_path);

        Ok(())
    });
}

/// Redirect stderr to stdout through dup2 action and verify merged output.
#[cfg(unix)]
#[test]
fn test_process_spawn_with_actions_dup2_stderr_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let output_path = unique_temp_file_path("SPAWN_DUP2_STDERR");
        let output_path_text = output_path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&output_path);

        let output_file = std::fs::File::create(&output_path).map_err(|error| {
            RuntimeError::from(PlatformError::io(format!(
                "failed to create output file: {error}"
            )))
            .boxed()
        })?;
        let output_descriptor = output_file.as_raw_fd();

        let options = ProcessSpawnOptionsSpec {
            cwd: cwd.clone(),
            detached: false,
            reset_signals: true,
            new_process_group: false,
        };
        let command = "/bin/sh".to_string();
        let arguments = vec!["-c".to_string(), "printf out; printf err 1>&2".to_string()];
        let environment = Vec::new();
        let stdio = vec![
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Descriptor,
                descriptor: output_descriptor,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
            },
        ];
        let actions = vec![ProcessFdActionSpec {
            op: ProcessFdActionKind::Dup2,
            source: 1,
            target: 2,
            path: output_path_text.clone(),
            flags: fs::OpenFlags(0),
            mode: fs::FileMode(0),
        }];

        let handle = context.spawn_with_actions(
            &command,
            &arguments,
            &environment,
            &options,
            &stdio,
            &actions,
        )?;
        let status = context.wait(handle, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 0);

        // output file should contain both stdout and redirected stderr payload
        drop(output_file);
        let output = std::fs::read_to_string(&output_path).expect("output file should be readable");
        assert_eq!(output, "outerr");
        let _ = std::fs::remove_file(&output_path);

        Ok(())
    });
}

/// Wait on a process-fd opened from a spawned child process id.
#[cfg(any(unix, windows))]
#[test]
fn test_process_fd_wait_spawn_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 37);
        let child_pid = spawn_shell(command, arguments)?;

        let handle = context.process_fd_open(child_pid, ProcessFdFlags(0))?;
        let status = context.process_fd_wait(handle, 2_000_000_000)?;

        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 37);
        context.process_fd_close(handle)?;

        Ok(())
    });
}

/// Wait a process-fd after child exit delay and still observe terminal status.
#[cfg(any(unix, windows))]
#[test]
fn test_process_fd_wait_after_exit_delay_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 41);
        let child_pid = spawn_shell(command, arguments)?;
        let handle = context.process_fd_open(child_pid, ProcessFdFlags(0))?;

        // allow the child process to exit before waiting through the process-fd
        std::thread::sleep(std::time::Duration::from_millis(1500));

        // wait should still return the terminal exit status
        let status = context.process_fd_wait(handle, 2_000_000_000)?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 41);
        context.process_fd_close(handle)?;

        Ok(())
    });
}

/// Poll a process-fd, send a no-op signal, and await completion.
#[cfg(unix)]
#[test]
fn test_process_fd_try_wait_and_send_signal_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 0);
        let child_pid = spawn_shell(command, arguments)?;

        let handle = context.process_fd_open(child_pid, ProcessFdFlags(0))?;

        // try-wait should either report running/exited or a would-block error
        let first_try_wait = context.process_fd_try_wait(handle);
        match first_try_wait {
            Ok(status) => assert!(matches!(
                status.kind,
                ProcessWaitKind::Running | ProcessWaitKind::Exited
            )),
            Err(error) => {
                assert!(is_would_block(&error));
            }
        }

        context.process_fd_send_signal(handle, Signal(0), ProcessFdSignalFlags(0))?;

        let _ = context.process_fd_wait(handle, 3_000_000_000)?;
        context.process_fd_close(handle)?;

        Ok(())
    });
}

/// Report specific errors for invalid process-fd arguments and handles.
#[cfg(unix)]
#[test]
fn test_process_fd_validation_errors_are_specific() {
    with_harness_context(|mut context| {
        let pid = context.pid()?;

        // invalid flags and handles should be rejected with invalid-argument errors
        assert_platform_error_code(
            context.process_fd_open(pid, ProcessFdFlags(1)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let handle = context.process_fd_open(pid, ProcessFdFlags(0))?;
        assert_platform_error_code(
            context.process_fd_send_signal(handle, Signal(0), ProcessFdSignalFlags(1)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.process_fd_close(handle)?;

        let invalid_handle = ProcessFdHandle(ResourceId(0));
        assert_platform_error_code(
            context.process_fd_try_wait(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.process_fd_close(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.process_fd_send_signal(invalid_handle, Signal(0), ProcessFdSignalFlags(0)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // opening an unreachable pid should report one of the expected process errors
        assert_platform_error_codes(
            context.process_fd_open(ProcessId(i32::MAX as u32), ProcessFdFlags(0)),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::ProcessPermissionDenied,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Process,
            ],
        )?;

        let (command, arguments) = shell_sleep_then_exit_command(1, 0);
        let child_pid = spawn_shell(command, arguments)?;
        let timeout_handle = context.process_fd_open(child_pid, ProcessFdFlags(0))?;
        let timeout_status = context.process_fd_wait(timeout_handle, u64::MAX)?;
        assert_eq!(timeout_status.kind, ProcessWaitKind::Exited);
        context.process_fd_close(timeout_handle)?;

        Ok(())
    });
}
