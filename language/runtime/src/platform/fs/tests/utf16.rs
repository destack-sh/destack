#[cfg(windows)]
mod windows_tests {
    use super::super::{temp_dir, with_harness_context};
    use crate::platform::fs::{FileMode, OpenFlags};

    #[test]
    fn test_fs_utf16_open_rename_unlink() {
        with_harness_context(|mut context| {
            // runtime and temp directory
            let temp_dir = temp_dir("fs_utf16");
            let file_a = temp_dir.join("alpha.txt");
            let file_b = temp_dir.join("beta.txt");

            let dir = context.path_bytes(&temp_dir);
            context.mkdir(dir, FileMode(0o755))?;

            // open using utf16 path
            let file = context.path_utf16(&file_a);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let handle = context.open(file, flags, FileMode(0o644))?;
            context.close(handle)?;

            // stat and rename using utf16
            let file = context.path_utf16(&file_a);
            let stat = context.stat(file)?;
            assert_eq!(stat.size.0, 0);

            let from = context.path_utf16(&file_a);
            let to = context.path_utf16(&file_b);
            context.rename(from, to)?;

            // cleanup
            let to = context.path_utf16(&file_b);
            context.unlink(to)?;

            let dir = context.path_bytes(&temp_dir);
            context.rmdir(dir)?;

            Ok(())
        });
    }
}
