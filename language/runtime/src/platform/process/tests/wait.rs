#[cfg(unix)]
use super::{fork_child_exit, fork_child_sleep_then_exit};
use super::{shell_exit_command, shell_sleep_then_exit_command, spawn_shell, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::process::Signal;
use crate::platform::process::{ProcessWaitFlags, ProcessWaitKind};

#[cfg(unix)]
#[test]
fn test_process_wait_pid_exit_status_roundtrip() {
    with_harness_context(|mut context| {
        let child_pid = fork_child_exit(7)?;
        let status = context.wait_pid(child_pid, ProcessWaitFlags(0))?;

        assert_eq!(status.pid, child_pid);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 7);
        assert_eq!(status.signal, Signal(0));
        assert!(!status.core_dumped);

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_process_wait_pid_nohang_would_block_then_reap() {
    with_harness_context(|mut context| {
        let child_pid = fork_child_sleep_then_exit(1, 0)?;

        let first = context.wait_pid(child_pid, ProcessWaitFlags(libc::WNOHANG as u32));
        let first = first.expect_err("first nonblocking wait should report would-block");
        let first = first
            .platform_error()
            .expect("error should be platform error");
        assert_eq!(first.code, PlatformErrorCode::IoWouldBlock);

        let status = context.wait_pid(child_pid, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_process_wait_pid_spawn_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_exit_command(21);
        let child_pid = spawn_shell(command, arguments)?;
        let status = context.wait_pid(child_pid, ProcessWaitFlags(0))?;

        assert_eq!(status.pid, child_pid);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, 21);

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_process_wait_pid_nonblocking_spawn_roundtrip() {
    let nohang = ProcessWaitFlags(crate::platform::process::core::PROCESS_WAIT_FLAG_NOHANG);

    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 0);
        let child_pid = spawn_shell(command, arguments)?;

        let first = context.wait_pid(child_pid, nohang);
        let first = first.expect_err("first nonblocking wait should report would-block");
        let first = first
            .platform_error()
            .expect("error should be platform error");
        assert_eq!(first.code, PlatformErrorCode::IoWouldBlock);

        let status = context.wait_pid(child_pid, ProcessWaitFlags(0))?;
        assert_eq!(status.kind, ProcessWaitKind::Exited);

        Ok(())
    });
}
