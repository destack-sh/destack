use super::{FsHarness, native_slice, native_slice_mut, path_bytes, temp_dir, with_native_harness};
use crate::platform::fs::{
    FileMode, FileOffset, OpenFlags, destack_fs_close, destack_fs_mkdir_bytes,
    destack_fs_open_bytes, destack_fs_read, destack_fs_rmdir_bytes, destack_fs_unlink_bytes,
    destack_fs_write,
};
use crate::platform::resource::{FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_open_read_write_close() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_basic");
        let file_path = temp_dir.join("data.txt");

        // create directory and open file
        let handle = harness.with_context(|| {
            let (_bytes, path) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(path, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            handle
        });

        // write to the file
        harness.with_context(|| {
            let buffer = b"hello world".to_vec();
            let slice = native_slice(&buffer);
            let mut out = 0u64;
            let status = unsafe { destack_fs_write(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "write");
            assert_eq!(out, buffer.len() as u64);
        });

        // read back
        let read_back = harness.with_context(|| {
            let mut buffer = vec![0u8; 32];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_fs_read(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "read");
            buffer.truncate(out as usize);
            buffer
        });
        assert_eq!(read_back, b"hello world");

        // close handle and remove file/dir
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
