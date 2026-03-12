#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{assert_platform_error_codes_with_privileged_policy, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{AccessMode, AtFlags, FileMode, OpenFlags};
use crate::tests::platform::is_privileged_test_mode;
use std::path::Path;

/// One unknown `AtFlags` bit for validation tests.
#[cfg(windows)]
const AT_UNKNOWN_FLAG: u32 = 0x10;

/// Check write access behavior before and after chmod mode changes.
#[cfg(any(unix, windows))]
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
            assert_platform_error_codes_with_privileged_policy(
                context.destack_fs_access(file, AccessMode(0o222)),
                &[PlatformErrorCode::IoPermissionDenied],
            )?;
        }

        #[cfg(windows)]
        {
            let file = context.path_bytes(&file_path);
            assert_platform_error_codes_with_privileged_policy(
                context.destack_fs_access(file, AccessMode(0o222)),
                &[PlatformErrorCode::IoPermissionDenied],
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

/// Check directory-relative access with one opened directory handle.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_accessat_roundtrip() {
    with_harness_context(|mut context| {
        // setup runtime and paths
        let temp_dir = temp_dir("fs_accessat");
        let file_name = Path::new("file.txt");

        // create directory and file through relative open
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // check existence through accessat
        let file = context.path_bytes(file_name);
        context.destack_fs_accessat(dir_handle, file, AccessMode(0), AtFlags(0))?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unknown accessat flags on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_accessat_rejects_unknown_flags_on_windows() {
    with_harness_context(|mut context| {
        // setup runtime and paths
        let temp_dir = temp_dir("fs_accessat_unknown_flags");
        let file_name = Path::new("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;

        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject unknown bits instead of reporting generic unsupported behavior
        let file = context.path_bytes(file_name);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_accessat(dir_handle, file, AccessMode(0), AtFlags(AT_UNKNOWN_FLAG)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Check directory-relative access through a utf16 path on unix hosts.
#[cfg(unix)]
#[test]
fn test_fs_accessat_utf16_roundtrip() {
    with_harness_context(|mut context| {
        // setup runtime and paths
        let temp_dir = temp_dir("fs_accessat_utf16");
        let file_name = Path::new("file.txt");

        // create directory and file through relative open
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // check existence through the utf16 accessat lane
        let file = context.path_utf16(file_name);
        context.destack_fs_accessat(dir_handle, file, AccessMode(0), AtFlags(0))?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
