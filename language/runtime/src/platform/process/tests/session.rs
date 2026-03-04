use super::{
    assert_platform_error_codes_with_privileged_policy, with_harness_context,
    with_native_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::ProcessId;

/// Report specific errors for setpgid with invalid process ids.
#[cfg(unix)]
#[test]
fn test_process_session_setpgid_invalid_pid_reports_specific_error() {
    with_harness_context(|mut context| {
        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_setpgid(ProcessId(u32::MAX), ProcessId(u32::MAX)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )
    });
}

/// Create a new session in a forked child and verify returned session id.
#[cfg(unix)]
#[test]
fn test_process_session_setsid_in_child() {
    with_native_harness_context(|mut context| {
        let child = unsafe { libc::fork() };
        if child == 0 {
            let setsid_result = context.destack_process_setsid();
            if let Ok(session_id) = setsid_result {
                let child_pid = unsafe { libc::getpid() as u32 };
                if session_id.0 == child_pid {
                    unsafe {
                        libc::_exit(0);
                    }
                }
            }
            unsafe {
                libc::_exit(111);
            }
        }

        // parent should observe successful child session creation
        assert!(child > 0);
        let mut status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        assert_eq!(waited, child);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);

        Ok(())
    });
}
