use super::{FsHarness, path_bytes, temp_dir, with_native_harness};
use crate::platform::fs::{
    AccessMode, FileMode, OpenFlags, destack_fs_access_bytes, destack_fs_chmod_bytes,
    destack_fs_chown_bytes, destack_fs_close, destack_fs_fchmod, destack_fs_fchown,
    destack_fs_futimes, destack_fs_lutimes_bytes, destack_fs_mkdir_bytes, destack_fs_open_bytes,
    destack_fs_rmdir_bytes, destack_fs_unlink_bytes, destack_fs_utimes_bytes,
};
use crate::platform::resource::{FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_access_and_chmod() {
    with_native_harness(|harness| {
        // setup runtime and paths
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_attrs");
        let file_path = temp_dir.join("file.txt");
        #[cfg(unix)]
        let should_check_access_failure = unsafe { libc::geteuid() != 0 };
        #[cfg(not(unix))]
        let should_check_access_failure = false;

        // create file
        harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close");
        });

        // check access with write permissions
        harness.with_context(|| {
            let (_bytes, file) = path_bytes(&file_path);
            let status = unsafe { destack_fs_access_bytes(file, AccessMode(0o222)) };
            runtime.assert_status_ok(status, "access write");

            let status = unsafe { destack_fs_chmod_bytes(file, FileMode(0o444)) };
            runtime.assert_status_ok(status, "chmod read-only");

            if should_check_access_failure {
                let status = unsafe { destack_fs_access_bytes(file, AccessMode(0o222)) };
                runtime.assert_status_err(status, "access write after chmod");
            }
        });

        // cleanup
        harness.with_context(|| {
            let (_bytes, file) = path_bytes(&file_path);
            let status = unsafe { destack_fs_unlink_bytes(file) };
            runtime.assert_status_ok(status, "unlink");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}

#[cfg(unix)]
#[test]
fn test_fs_chown_and_times() {
    with_native_harness(|harness| {
        // setup runtime and paths
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_chown");
        let file_path = temp_dir.join("file.txt");
        #[cfg(unix)]
        let can_chown = unsafe { libc::geteuid() == 0 };
        #[cfg(not(unix))]
        let can_chown = false;

        // create file and open handle
        let handle = harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            handle
        });

        // apply chown and time updates
        harness.with_context(|| {
            let (_bytes, file) = path_bytes(&file_path);
            if can_chown {
                let uid = unsafe { libc::getuid() };
                let gid = unsafe { libc::getgid() };
                let status = unsafe { destack_fs_chown_bytes(file, uid, gid) };
                runtime.assert_status_ok(status, "chown");

                let status = unsafe { destack_fs_fchown(handle, uid, gid) };
                runtime.assert_status_ok(status, "fchown");
            }

            let status = unsafe { destack_fs_utimes_bytes(file, 1_000_000, 2_000_000) };
            runtime.assert_status_ok(status, "utimes");

            let status = unsafe { destack_fs_lutimes_bytes(file, 3_000_000, 4_000_000) };
            runtime.assert_status_ok(status, "lutimes");

            let status = unsafe { destack_fs_futimes(handle, 5_000_000, 6_000_000) };
            runtime.assert_status_ok(status, "futimes");

            let status = unsafe { destack_fs_fchmod(handle, FileMode(0o600)) };
            runtime.assert_status_ok(status, "fchmod");
        });

        // cleanup
        harness.with_context(|| {
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close");

            let (_bytes, file) = path_bytes(&file_path);
            let status = unsafe { destack_fs_unlink_bytes(file) };
            runtime.assert_status_ok(status, "unlink");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
