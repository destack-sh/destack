use super::{temp_dir, with_harness_context};
use crate::platform::fs::{AtFlags, FileMode, OpenFlags};
use std::path::Path;

fn is_privileged_test_mode() -> bool {
    let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Change ownership with chown and fchown and verify the resulting stat ids.
#[cfg(unix)]
#[test]
fn test_fs_chown_and_fchown_succeed_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_harness_context(|mut context| {
        let temp_directory = temp_dir("fs_privileged_chown");
        let file_path = temp_directory.join("file.txt");

        let directory = context.path_bytes(&temp_directory);
        context.mkdir(directory, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o600))?;

        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        let file = context.path_bytes(&file_path);
        context.chown(file.clone(), uid, gid)?;
        context.fchown(handle, uid, gid)?;

        // ownership should match the requested ids after privileged updates
        let stat = context.stat(file.clone())?;
        assert_eq!(stat.uid, uid);
        assert_eq!(stat.gid, gid);

        context.close(handle)?;
        context.unlink(file)?;
        let directory = context.path_bytes(&temp_directory);
        context.rmdir(directory)?;

        Ok(())
    });
}

/// Change ownership with fchownat and verify the resulting statat ids.
#[cfg(unix)]
#[test]
fn test_fs_fchownat_succeeds_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_harness_context(|mut context| {
        let temp_directory = temp_dir("fs_privileged_fchownat");
        let file_name = Path::new("file.txt");

        let directory = context.path_bytes(&temp_directory);
        context.mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_directory);
        let dir_handle = context.opendir(directory)?;

        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.openat(dir_handle, file, flags, FileMode(0o600))?;
        context.close(handle)?;

        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        let file = context.path_bytes(file_name);
        context.fchownat(dir_handle, file, uid, gid, AtFlags(0))?;

        // ownership should match the requested ids after privileged updates
        let file = context.path_bytes(file_name);
        let stat = context.statat(dir_handle, file, AtFlags(0))?;
        assert_eq!(stat.uid, uid);
        assert_eq!(stat.gid, gid);

        let file = context.path_bytes(file_name);
        context.unlinkat(dir_handle, file, AtFlags(0))?;
        context.closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_directory);
        context.rmdir(directory)?;

        Ok(())
    });
}
