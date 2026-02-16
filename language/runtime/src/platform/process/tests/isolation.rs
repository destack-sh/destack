use super::{
    assert_platform_error_code, assert_platform_error_codes, unique_env_name, with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{
    ProcessId, ProcessNamespaceKind, ProcessUnshareFlags, SyscallFilterFlags,
};

/// Change the working directory and restore the original directory afterward.
#[cfg(unix)]
#[test]
fn test_process_chdir_updates_cwd_and_restores() {
    with_harness_context(|mut context| {
        let original_cwd = context.cwd()?;
        let target_path = std::env::temp_dir().join(unique_env_name("CHDIR"));
        std::fs::create_dir_all(&target_path).expect("temporary chdir directory should be created");
        let target_path = target_path
            .to_str()
            .expect("temporary chdir path should be utf-8")
            .to_string();

        let mut observed_cwd = None;
        let change_result: RuntimeResult<()> = (|| {
            context.chdir(&target_path)?;
            observed_cwd = Some(context.cwd()?);
            Ok(())
        })();

        context.chdir(&original_cwd)?;

        // observed cwd should match the canonical target directory
        change_result?;
        let observed_cwd = observed_cwd.expect("chdir should record observed cwd");
        let observed_cwd = std::fs::canonicalize(observed_cwd).expect("observed cwd should exist");
        let target_cwd = std::fs::canonicalize(&target_path).expect("target cwd should exist");
        assert_eq!(observed_cwd, target_cwd);
        let _ = std::fs::remove_dir_all(&target_path);

        Ok(())
    });
}

/// Report specific errors for chroot with a missing path.
#[cfg(unix)]
#[test]
fn test_process_chroot_missing_path_reports_specific_error() {
    with_harness_context(|mut context| {
        assert_platform_error_codes(
            context.chroot("/definitely/missing/destack-process-chroot"),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::ProcessPermissionDenied,
            ],
        )
    });
}

/// Reject syscall filter installs with an invalid program shape.
#[cfg(unix)]
#[test]
fn test_process_install_syscall_filter_validates_program_shape() {
    with_harness_context(|mut context| {
        assert_platform_error_codes(
            context.install_syscall_filter(&[], SyscallFilterFlags(0)),
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )
    });
}

/// Reject empty host names for set_host_name.
#[cfg(unix)]
#[test]
fn test_process_set_host_name_rejects_empty_value() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.set_host_name(""),
            PlatformErrorCode::InvalidArgumentValue,
        )
    });
}

/// Report specific errors for set_network_namespace with a missing path.
#[cfg(unix)]
#[test]
fn test_process_set_network_namespace_missing_path_reports_specific_error() {
    with_harness_context(|mut context| {
        assert_platform_error_codes(
            context.set_network_namespace("/definitely/missing/destack-process-netns"),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::ProcessPermissionDenied,
                PlatformErrorCode::NotSupported,
            ],
        )
    });
}

/// Report specific errors for setns with a missing target process.
#[cfg(unix)]
#[test]
fn test_process_setns_missing_process_reports_specific_error() {
    with_harness_context(|mut context| {
        assert_platform_error_codes(
            context.setns(ProcessId(u32::MAX), ProcessNamespaceKind::Network),
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::ProcessPermissionDenied,
                PlatformErrorCode::NotSupported,
            ],
        )
    });
}

/// Reject unshare calls whose flag word exceeds supported width.
#[cfg(unix)]
#[test]
fn test_process_unshare_rejects_oversized_flag_word() {
    with_harness_context(|mut context| {
        assert_platform_error_codes(
            context.unshare(ProcessUnshareFlags(u64::MAX)),
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )
    });
}
