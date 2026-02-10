#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{temp_dir, with_harness_context};
use crate::platform::fs::{FileMode, FileOffset, OpenFlags, SeekWhence};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_truncate_and_fsync() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_truncate");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;

        let buffer = b"hello world".to_vec();
        context.pwrite(handle, &buffer, FileOffset(0))?;
        context.fsync(handle)?;
        context.ftruncate(handle, FileOffset(5))?;

        let mut buffer = vec![0u8; 16];
        let out = context.pread(handle, &mut buffer, FileOffset(0))?;
        buffer.truncate(out as usize);
        assert_eq!(buffer, b"hello");

        // cleanup
        context.close(handle)?;

        let file = context.path_bytes(&file_path);
        context.truncate(file.clone(), FileOffset(0))?;
        context.unlink(file)?;

        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_fs_readv_writev() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_vectored");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;

        let buf_a = b"hello ".to_vec();
        let buf_b = b"world".to_vec();
        let buffers = [&buf_a[..], &buf_b[..]];
        let out = context.writev(handle, &buffers)?;
        assert_eq!(out, 11);

        let _ = context.seek(handle, FileOffset(0), SeekWhence::Set)?;

        let mut buf_a = vec![0u8; 6];
        let mut buf_b = vec![0u8; 5];
        let mut buffers = [&mut buf_a[..], &mut buf_b[..]];
        let out = context.readv(handle, &mut buffers)?;
        let mut combined = Vec::new();
        combined.extend_from_slice(&buf_a);
        combined.extend_from_slice(&buf_b);
        combined.truncate(out as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_preadv_pwritev() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_preadv");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // write payload using pwritev
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;

        let buf_a = b"hello ".to_vec();
        let buf_b = b"world".to_vec();
        let buffers = [&buf_a[..], &buf_b[..]];
        let out = context.pwritev(handle, &buffers, FileOffset(0))?;
        assert_eq!(out, 11);

        // read back using preadv
        let mut buf_a = vec![0u8; 6];
        let mut buf_b = vec![0u8; 5];
        let mut buffers = [&mut buf_a[..], &mut buf_b[..]];
        let out = context.preadv(handle, &mut buffers, FileOffset(0))?;
        let mut combined = Vec::new();
        combined.extend_from_slice(&buf_a);
        combined.extend_from_slice(&buf_b);
        combined.truncate(out as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
