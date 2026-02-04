use super::{FsHarness, native_slice_mut, path_bytes, temp_dir, with_native_harness};
use crate::platform::fs::{
    FileMode, FileOffset, destack_fs_close, destack_fs_closedir, destack_fs_mkdir_bytes,
    destack_fs_opendir_bytes, destack_fs_read, destack_fs_rmdir_bytes,
};
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_invalid_file_handle() {
    with_native_harness(|harness| {
        // setup runtime
        let runtime = harness.runtime();

        harness.with_context(|| {
            let mut buffer = vec![0u8; 16];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe {
                destack_fs_read(&mut out, FileHandle(ResourceId(9999)), slice, FileOffset(0))
            };
            runtime.assert_status_err(status, "read invalid handle");

            let status = unsafe { destack_fs_close(FileHandle(ResourceId(9999))) };
            runtime.assert_status_err(status, "close invalid handle");
        });
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_invalid_directory_handle() {
    with_native_harness(|harness| {
        // setup runtime and directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_dir_invalid");
        let sub_dir = temp_dir.join("child");

        let handle = harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, child) = path_bytes(&sub_dir);
            let status = unsafe { destack_fs_mkdir_bytes(child, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir child");

            let mut handle = DirectoryHandle(ResourceId(0));
            let status = unsafe { destack_fs_opendir_bytes(&mut handle, dir) };
            runtime.assert_status_ok(status, "opendir");
            handle
        });

        harness.with_context(|| {
            let status = unsafe { destack_fs_closedir(handle) };
            runtime.assert_status_ok(status, "closedir");

            let status = unsafe { destack_fs_closedir(handle) };
            runtime.assert_status_err(status, "closedir after close");

            let status = unsafe { destack_fs_closedir(DirectoryHandle(ResourceId(9999))) };
            runtime.assert_status_err(status, "closedir invalid handle");
        });

        harness.with_context(|| {
            let (_bytes, child) = path_bytes(&sub_dir);
            let status = unsafe { destack_fs_rmdir_bytes(child) };
            runtime.assert_status_ok(status, "rmdir child");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
