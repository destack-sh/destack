#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{temp_dir, with_harness_context};
use crate::platform::fs::{FileMode, FileOffset, OpenFlags, SeekWhence};

/// Truncate a file and persist the change with fsync.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_truncate_and_fsync() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_truncate");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buffer = b"hello world".to_vec();
        let buffer_value = context.bytes_slice_value(&buffer)?;
        context.destack_fs_pwrite(handle, buffer_value, FileOffset(0))?;
        context.destack_fs_fsync(handle)?;
        context.destack_fs_ftruncate(handle, FileOffset(5))?;

        let buffer = context.zeroed_bytes_slice_value(16)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"hello");

        // cleanup
        context.destack_fs_close(handle)?;

        let file = context.path_bytes(&file_path);
        context.destack_fs_truncate(file, FileOffset(0))?;
        context.destack_fs_unlink(file)?;

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Write and read file data through vectored io calls.
#[cfg(unix)]
#[test]
fn test_fs_readv_writev() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_vectored");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buf_a = b"hello ".to_vec();
        let buf_b = b"world".to_vec();
        let buffers = [&buf_a[..], &buf_b[..]];
        let buffers = context.bytes_slices_value(&buffers)?;
        let out = context.destack_fs_writev(handle, buffers)?;
        assert_eq!(out, 11);

        let _ = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::Set)?;

        let mut buffers = vec![vec![0u8; 6], vec![0u8; 5]];
        let buffers = context.mutable_bytes_slices_value(&mut buffers)?;
        let (buffers_call, buffers_value) = context.duplicate_value(buffers);
        let out = context.destack_fs_readv(handle, buffers_call)?;
        let mut combined = context.bytes_slices_from_value(buffers_value)?.concat();
        combined.truncate(out as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Write and read file data through positional vectored io calls.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_preadv_pwritev() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_preadv");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload using pwritev
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buf_a = b"hello ".to_vec();
        let buf_b = b"world".to_vec();
        let buffers = [&buf_a[..], &buf_b[..]];
        let buffers = context.bytes_slices_value(&buffers)?;
        let out = context.destack_fs_pwritev(handle, buffers, FileOffset(0))?;
        assert_eq!(out, 11);

        // read back using preadv
        let mut buffers = vec![vec![0u8; 6], vec![0u8; 5]];
        let buffers = context.mutable_bytes_slices_value(&mut buffers)?;
        let (buffers_call, buffers_value) = context.duplicate_value(buffers);
        let out = context.destack_fs_preadv(handle, buffers_call, FileOffset(0))?;
        let mut combined = context.bytes_slices_from_value(buffers_value)?.concat();
        combined.truncate(out as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
