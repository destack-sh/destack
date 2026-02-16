#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{FsHarnessKind, assert_platform_error_codes, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs as platform_fs;
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
        context.mkdir(dir, FileMode(0o755))?;
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        // check access with write permissions
        let file = context.path_bytes(&file_path);
        context.access(file.clone(), AccessMode(0o222))?;
        context.chmod(file.clone(), FileMode(0o444))?;

        if should_check_access_failure {
            if is_privileged_test_mode() {
                panic!(
                    "DESTACK_TEST_PRIVILEGED=1 requires privileged coverage, but access write check is running in unprivileged mode",
                );
            }

            let file = context.path_bytes(&file_path);
            match context.kind() {
                FsHarnessKind::Native => {
                    let native_path = file
                        .native()
                        .expect("native path required for native access");
                    let status = unsafe {
                        platform_fs::destack_fs_attrs_access(native_path, AccessMode(0o222))
                    };
                    assert_platform_error_codes(
                        context.status_result(status, "access write after chmod"),
                        &[PlatformErrorCode::IoPermissionDenied, PlatformErrorCode::Io],
                    )?;
                }
                FsHarnessKind::Vm => {
                    assert_platform_error_codes(
                        context.access(file, AccessMode(0o222)),
                        &[PlatformErrorCode::IoPermissionDenied, PlatformErrorCode::Io],
                    )?;
                }
            }
        }

        // cleanup
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

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
        context.mkdir(dir, FileMode(0o755))?;
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;

        // apply chown and time updates
        let file = context.path_bytes(&file_path);
        if can_chown {
            let uid = unsafe { libc::getuid() };
            let gid = unsafe { libc::getgid() };
            context.chown(file.clone(), uid, gid)?;
            context.fchown(handle, uid, gid)?;
        } else if is_privileged_test_mode() {
            panic!(
                "DESTACK_TEST_PRIVILEGED=1 requires chown coverage, but test process lacks privileges",
            );
        }

        context.utimes(file.clone(), 1_000_000, 2_000_000)?;
        context.lutimes(file.clone(), 3_000_000, 4_000_000)?;
        context.futimes(handle, 5_000_000, 6_000_000)?;
        context.fchmod(handle, FileMode(0o600))?;

        // cleanup
        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
