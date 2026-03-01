use super::{
    ProcessWaitKind, shell_exit_command, shell_sleep_then_exit_command, spawn_shell,
    with_harness_context,
};
#[cfg(unix)]
use super::{fork_child_exit, fork_child_sleep_then_exit};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{ProcessId, ProcessWaitFlags, host as host_process};

/// Wait for a forked child exit and verify full wait status fields.
#[cfg(unix)]
#[test]
fn test_process_wait_pid_exit_status_roundtrip() {
    with_harness_context(|mut context| {
        let child_pid = fork_child_exit(7)?;
        let status = context.destack_process_wait_pid(child_pid, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);

        assert_eq!(status.pid, child_pid);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(7));
        assert_eq!(status.signal, None);
        assert!(!status.core_dumped);

        Ok(())
    });
}

/// Observe would-block for nohang waits before reaping child exit.
#[cfg(unix)]
#[test]
fn test_process_wait_pid_nohang_would_block_then_reap() {
    with_harness_context(|mut context| {
        let child_pid = fork_child_sleep_then_exit(1, 0)?;

        let first =
            context.destack_process_wait_pid(child_pid, ProcessWaitFlags(libc::WNOHANG as u32));
        // nonblocking wait should report would-block while child is still running
        let first = first.expect_err("first nonblocking wait should report would-block");
        let first = first
            .platform_error()
            .expect("error should be platform error");
        assert_eq!(first.code, PlatformErrorCode::IoWouldBlock);

        let status = context.destack_process_wait_pid(child_pid, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);

        Ok(())
    });
}

/// Wait for a shell-spawned child process and verify exit status.
#[cfg(any(unix, windows))]
#[test]
fn test_process_wait_pid_spawn_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_exit_command(21);
        let child_pid = spawn_shell(command, arguments)?;
        let status = context.destack_process_wait_pid(child_pid, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);

        assert_eq!(status.pid, child_pid);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(21));

        Ok(())
    });
}

/// Observe nonblocking would-block behavior for a spawned child process.
#[cfg(any(unix, windows))]
#[test]
fn test_process_wait_pid_nonblocking_spawn_roundtrip() {
    let nohang = ProcessWaitFlags(host_process::PROCESS_WAIT_FLAG_NOHANG);

    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 0);
        let child_pid = spawn_shell(command, arguments)?;

        let first = context.destack_process_wait_pid(child_pid, nohang);
        // nonblocking wait should report would-block while child is still running
        let first = first.expect_err("first nonblocking wait should report would-block");
        let first = first
            .platform_error()
            .expect("error should be platform error");
        assert_eq!(first.code, PlatformErrorCode::IoWouldBlock);

        let status = context.destack_process_wait_pid(child_pid, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);

        Ok(())
    });
}

/// Reject waitpid calls that use invalid flag bits.
#[cfg(unix)]
#[test]
fn test_process_wait_pid_rejects_invalid_flags() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;
        let invalid = context.destack_process_wait_pid(pid, ProcessWaitFlags(u32::MAX));
        let invalid = invalid.expect_err("invalid wait flags should fail");
        let invalid = invalid
            .platform_error()
            .expect("error should be platform error");
        assert_eq!(invalid.code, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Reject waitpid calls that use pid zero.
#[cfg(any(unix, windows))]
#[test]
fn test_process_wait_pid_rejects_zero_pid() {
    with_harness_context(|mut context| {
        let invalid = context.destack_process_wait_pid(ProcessId(0), ProcessWaitFlags(0));
        let invalid = invalid.expect_err("pid zero should fail");
        let invalid = invalid
            .platform_error()
            .expect("error should be platform error");
        assert_eq!(invalid.code, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
