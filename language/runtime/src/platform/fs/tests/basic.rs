use super::{temp_dir, with_harness_context};
use crate::platform::fs::{FileMode, FileOffset, OpenFlags};

/// Open, write, read, and close a file through the harness.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_open_read_write_close() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_basic");
        let file_path = temp_dir.join("data.txt");

        // create directory and open file
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        // write to the file
        let buffer = b"hello world".to_vec();
        let buffer_value = context.bytes_slice_value(&buffer)?;
        let out = context.destack_fs_pwrite(handle, buffer_value, FileOffset(0))?;
        assert_eq!(out, buffer.len() as u64);

        // read back
        let buffer = context.zeroed_bytes_slice_value(32)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"hello world");

        // cleanup
        context.destack_fs_close(handle)?;

        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
