use super::{FsHarness, native_slice, native_slice_mut, path_bytes, temp_dir, with_native_harness};
use crate::platform::NativeSlice;
use crate::platform::fs::{
    FileMode, FileOffset, OpenFlags, destack_fs_dir_mkdir, destack_fs_dir_rmdir,
    destack_fs_file_close, destack_fs_file_fsync, destack_fs_file_ftruncate, destack_fs_file_open,
    destack_fs_file_pread, destack_fs_file_preadv, destack_fs_file_pwrite, destack_fs_file_pwritev,
    destack_fs_file_truncate, destack_fs_path_unlink,
};
use crate::platform::resource::{FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_truncate_and_fsync() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_truncate");
        let file_path = temp_dir.join("data.txt");

        let handle = harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_mkdir(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status = unsafe { destack_fs_file_open(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            handle
        });

        harness.with_context(|| {
            let buffer = b"hello world".to_vec();
            let slice = native_slice(&buffer);
            let mut out = 0u64;
            let status = unsafe { destack_fs_file_pwrite(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "write");

            let status = unsafe { destack_fs_file_fsync(handle) };
            runtime.assert_status_ok(status, "fsync");

            let status = unsafe { destack_fs_file_ftruncate(handle, FileOffset(5)) };
            runtime.assert_status_ok(status, "ftruncate");
        });

        let truncated = harness.with_context(|| {
            let mut buffer = vec![0u8; 16];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_fs_file_pread(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "read");
            buffer.truncate(out as usize);
            buffer
        });
        assert_eq!(truncated, b"hello");

        harness.with_context(|| {
            let status = unsafe { destack_fs_file_close(handle) };
            runtime.assert_status_ok(status, "close");

            let (_bytes, file) = path_bytes(&file_path);
            let status = unsafe { destack_fs_file_truncate(file, FileOffset(0)) };
            runtime.assert_status_ok(status, "truncate");

            let status = unsafe { destack_fs_path_unlink(file) };
            runtime.assert_status_ok(status, "unlink");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_rmdir(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_readv_writev() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_vectored");
        let file_path = temp_dir.join("data.txt");

        let handle = harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_mkdir(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status = unsafe { destack_fs_file_open(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            handle
        });

        harness.with_context(|| {
            let mut buf_a = b"hello ".to_vec();
            let mut buf_b = b"world".to_vec();
            let mut segments = [
                NativeSlice {
                    data: buf_a.as_mut_ptr(),
                    len: buf_a.len() as u32,
                },
                NativeSlice {
                    data: buf_b.as_mut_ptr(),
                    len: buf_b.len() as u32,
                },
            ];
            let buffers = NativeSlice {
                data: segments.as_mut_ptr(),
                len: segments.len() as u32,
            };
            let mut out = 0u64;
            let status =
                unsafe { destack_fs_file_pwritev(&mut out, handle, buffers, FileOffset(0)) };
            runtime.assert_status_ok(status, "writev");
            assert_eq!(out, 11);
        });

        let read_back = harness.with_context(|| {
            let mut buf_a = vec![0u8; 6];
            let mut buf_b = vec![0u8; 5];
            let mut segments = [
                NativeSlice {
                    data: buf_a.as_mut_ptr(),
                    len: buf_a.len() as u32,
                },
                NativeSlice {
                    data: buf_b.as_mut_ptr(),
                    len: buf_b.len() as u32,
                },
            ];
            let buffers = NativeSlice {
                data: segments.as_mut_ptr(),
                len: segments.len() as u32,
            };
            let mut out = 0u64;
            let status =
                unsafe { destack_fs_file_preadv(&mut out, handle, buffers, FileOffset(0)) };
            runtime.assert_status_ok(status, "readv");
            let mut combined = Vec::new();
            combined.extend_from_slice(&buf_a);
            combined.extend_from_slice(&buf_b);
            combined.truncate(out as usize);
            combined
        });

        assert_eq!(read_back, b"hello world");

        harness.with_context(|| {
            let status = unsafe { destack_fs_file_close(handle) };
            runtime.assert_status_ok(status, "close");

            let (_bytes, file) = path_bytes(&file_path);
            let status = unsafe { destack_fs_path_unlink(file) };
            runtime.assert_status_ok(status, "unlink");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_rmdir(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
