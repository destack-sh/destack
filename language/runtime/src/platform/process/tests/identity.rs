use super::{
    assert_platform_error_codes, syscall_getpgid, syscall_group_ids, syscall_groups,
    syscall_user_ids, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{ProcessId, Signal};

/// Send signal zero to the current process without delivering a signal.
#[cfg(unix)]
#[test]
fn test_process_kill_zero_to_self() {
    let pid = ProcessId(std::process::id());

    with_harness_context(|mut context| {
        context.kill(pid, Signal(0))?;
        Ok(())
    });
}

/// Match scalar identity reads against direct syscall results.
#[cfg(unix)]
#[test]
fn test_process_identity_scalar_syscall_parity() {
    let expected_pid = ProcessId(std::process::id());
    let expected_ppid = ProcessId(unsafe { libc::getppid() as u32 });
    let expected_uid = unsafe { libc::getuid() as u32 };
    let expected_gid = unsafe { libc::getgid() as u32 };
    let expected_euid = unsafe { libc::geteuid() as u32 };
    let expected_egid = unsafe { libc::getegid() as u32 };

    with_harness_context(|mut context| {
        // scalar identity values should match direct syscall observations
        assert_eq!(context.pid()?, expected_pid);
        assert_eq!(context.ppid()?, expected_ppid);
        assert_eq!(context.uid()?.0, expected_uid);
        assert_eq!(context.gid()?.0, expected_gid);
        assert_eq!(context.euid()?.0, expected_euid);
        assert_eq!(context.egid()?.0, expected_egid);
        Ok(())
    });
}

/// Match extended identity reads against direct syscall results.
#[cfg(unix)]
#[test]
fn test_process_identity_extended_syscall_parity() {
    let expected_group_ids = syscall_group_ids().expect("group ids syscall should succeed");
    let expected_groups = syscall_groups().expect("groups syscall should succeed");
    let expected_user_ids = syscall_user_ids().expect("user ids syscall should succeed");

    with_harness_context(|mut context| {
        // extended identity vectors should match direct syscall observations
        assert_eq!(context.group_ids()?, expected_group_ids);
        assert_eq!(context.groups()?, expected_groups);
        assert_eq!(context.user_ids()?, expected_user_ids);
        Ok(())
    });
}

/// Match getpgid results against direct syscall values.
#[cfg(unix)]
#[test]
fn test_process_getpgid_matches_syscall() {
    let pid = ProcessId(std::process::id());
    let expected = syscall_getpgid(pid).expect("getpgid syscall should succeed");

    with_harness_context(|mut context| {
        assert_eq!(context.getpgid(pid)?, expected);
        Ok(())
    });
}

/// Preserve identity state when setters are called with current values.
#[cfg(unix)]
#[test]
fn test_process_identity_setters_preserve_identity_state() {
    with_harness_context(|mut context| {
        // baseline identity snapshot
        let current_uid = context.uid()?;
        let current_gid = context.gid()?;
        let current_user_ids = context.user_ids()?;
        let current_group_ids = context.group_ids()?;
        let current_groups = context.groups()?;

        // setter outcomes: allowed values are success, permission denied, or not supported
        let set_uid = context.set_uid(current_uid);
        if let Err(error) = set_uid {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        // identity reads should remain unchanged after each setter call
        let observed_uid = context.uid()?;
        assert_eq!(observed_uid, current_uid);

        let set_euid = context.set_euid(current_user_ids.effective);
        if let Err(error) = set_euid {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        let observed_user_ids = context.user_ids()?;
        assert_eq!(observed_user_ids, current_user_ids);

        let set_gid = context.set_gid(current_gid);
        if let Err(error) = set_gid {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        let observed_gid = context.gid()?;
        assert_eq!(observed_gid, current_gid);

        let set_egid = context.set_egid(current_group_ids.effective);
        if let Err(error) = set_egid {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        let observed_group_ids = context.group_ids()?;
        assert_eq!(observed_group_ids, current_group_ids);

        let set_user_ids = context.set_user_ids(current_user_ids);
        if let Err(error) = set_user_ids {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        let observed_user_ids = context.user_ids()?;
        assert_eq!(observed_user_ids, current_user_ids);

        let set_group_ids = context.set_group_ids(current_group_ids);
        if let Err(error) = set_group_ids {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        let observed_group_ids = context.group_ids()?;
        assert_eq!(observed_group_ids, current_group_ids);

        let set_groups = context.set_groups(&current_groups);
        if let Err(error) = set_groups {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::ProcessPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        }

        let observed_groups = context.groups()?;
        assert_eq!(observed_groups, current_groups);

        Ok(())
    });
}
