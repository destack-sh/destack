use super::{temp_dir, with_harness_context};
use crate::platform::fs::{FileMode, FileOffset, OpenFlags};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_open_read_write_close() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_basic");
        let file_path = temp_dir.join("data.txt");

        // create directory and open file
        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;

        // write to the file
        let buffer = b"hello world".to_vec();
        let out = context.pwrite(handle, &buffer, FileOffset(0))?;
        assert_eq!(out, buffer.len() as u64);

        // read back
        let mut buffer = vec![0u8; 32];
        let out = context.pread(handle, &mut buffer, FileOffset(0))?;
        buffer.truncate(out as usize);
        assert_eq!(buffer, b"hello world");

        // cleanup
        context.close(handle)?;

        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
