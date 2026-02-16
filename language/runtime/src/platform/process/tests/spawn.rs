#[cfg(unix)]
use std::os::fd::AsRawFd;

use super::{
    ProcessFdActionSpec, ProcessSpawnOptionsSpec, ProcessStdioSpec, shell_exit_command,
    shell_sleep_then_exit_command, spawn_shell, with_harness_context,
};
#[cfg(unix)]
use super::{fork_child_sleep_then_exit, unique_temp_file_path};
#[cfg(unix)]
use crate::platform::fs;
#[cfg(unix)]
use crate::platform::process::ProcessFdActionKind;
use crate::platform::process::{
    ProcessFdFlags, ProcessStdioKind, ProcessWaitFlags, ProcessWaitKind,
};

#[cfg(unix)]
#[test]
fn test_process_spawn_wait_handle_roundtrip() {
    let cwd = crate::platform::process::core::process_cwd().expect("cwd should succeed");

    with_harness_context(|mut context| {
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let command = "/bin/sh";
        let arguments = vec!["-c".to_string(), "exit 17".to_string()];
        let environment = Vec::new();

        let handle = context.spawn(command, &arguments, &environment, &options)?;
        let status = context.wait(handle, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 17);

        let invalid = context.try_wait(handle);
        assert!(invalid.is_err());

        Ok(())
    });
}

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

#[cfg(any(unix, windows))]
#[test]
fn test_process_spawn_with_actions_wait_roundtrip() {
    let cwd = crate::platform::process::core::process_cwd().expect("cwd should succeed");

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

#[cfg(unix)]
#[test]
fn test_process_spawn_with_actions_open_stdout_roundtrip() {
    let cwd = crate::platform::process::core::process_cwd().expect("cwd should succeed");

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

        let output = std::fs::read_to_string(&output_path).expect("output file should be readable");
        assert_eq!(output, "open-action");
        let _ = std::fs::remove_file(&output_path);

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_process_spawn_with_actions_dup2_stderr_roundtrip() {
    let cwd = crate::platform::process::core::process_cwd().expect("cwd should succeed");

    with_harness_context(|mut context| {
        let output_path = unique_temp_file_path("SPAWN_DUP2_STDERR");
        let output_path_text = output_path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&output_path);

        let output_file = std::fs::File::create(&output_path).map_err(|error| {
            crate::diagnostic::RuntimeError::from(crate::platform::PlatformError::io(format!(
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

        drop(output_file);
        let output = std::fs::read_to_string(&output_path).expect("output file should be readable");
        assert_eq!(output, "outerr");
        let _ = std::fs::remove_file(&output_path);

        Ok(())
    });
}

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
