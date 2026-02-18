#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{assert_platform_error_codes, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{AccessMode, FileMode, OpenFlags};

fn is_privileged_test_mode() -> bool {
    let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Check write access behavior before and after chmod mode changes.
#[cfg(unix)]
#[test]
fn test_fs_access_and_chmod() {
    with_harness_context(|mut context| {
        // setup runtime and paths
        let temp_dir = temp_dir("fs_attrs");
        let file_path = temp_dir.join("file.txt");
        #[cfg(unix)]
        let should_check_access_failure = unsafe { libc::geteuid() != 0 };
        #[cfg(not(unix))]
        let should_check_access_failure = false;

        // create file
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // check access with write permissions
        let file = context.path_bytes(&file_path);
        context.destack_fs_access(file, AccessMode(0o222))?;
        context.destack_fs_chmod(file, FileMode(0o444))?;

        if should_check_access_failure {
            if is_privileged_test_mode() {
                panic!(
                    "DESTACK_TEST_PRIVILEGED=1 requires privileged coverage, but access write check is running in unprivileged mode",
                );
            }

            let file = context.path_bytes(&file_path);
            assert_platform_error_codes(
                context.destack_fs_access(file, AccessMode(0o222)),
                &[PlatformErrorCode::IoPermissionDenied, PlatformErrorCode::Io],
            )?;
        }

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Apply ownership, timestamp, and mode updates to one file handle.
#[cfg(unix)]
#[test]
fn test_fs_chown_and_times() {
    with_harness_context(|mut context| {
        // setup runtime and paths
        let temp_dir = temp_dir("fs_chown");
        let file_path = temp_dir.join("file.txt");
        #[cfg(unix)]
        let can_chown = unsafe { libc::geteuid() == 0 };
        #[cfg(not(unix))]
        let can_chown = false;

        // create file and open handle
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        // apply chown and time updates
        let file = context.path_bytes(&file_path);
        if can_chown {
            let uid = unsafe { libc::getuid() };
            let gid = unsafe { libc::getgid() };
            context.destack_fs_chown(file, uid, gid)?;
            context.destack_fs_fchown(handle, uid, gid)?;
        } else if is_privileged_test_mode() {
            panic!(
                "DESTACK_TEST_PRIVILEGED=1 requires chown coverage, but test process lacks privileges",
            );
        }

        context.destack_fs_utimes(file, 1_000_000, 2_000_000)?;
        context.destack_fs_lutimes(file, 3_000_000, 4_000_000)?;
        context.destack_fs_futimes(handle, 5_000_000, 6_000_000)?;
        context.destack_fs_fchmod(handle, FileMode(0o600))?;

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
