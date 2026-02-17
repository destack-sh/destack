#[cfg(windows)]
use crate::platform::fs::{FileMode, OpenFlags};

#[cfg(windows)]
mod windows_tests {
    use super::super::{temp_dir, with_harness_context};
    use super::{FileMode, OpenFlags};

    /// Open, rename, and unlink files through utf16 path bindings on windows.
    #[test]
    fn test_fs_utf16_open_rename_unlink() {
        with_harness_context(|mut context| {
            // runtime and temp directory
            let temp_dir = temp_dir("fs_utf16");
            let file_a = temp_dir.join("alpha.txt");
            let file_b = temp_dir.join("beta.txt");

            let dir = context.path_bytes(&temp_dir);
            context.destack_fs_mkdir(dir, FileMode(0o755))?;

            // open using utf16 path
            let file = context.path_utf16(&file_a);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
            context.destack_fs_close(handle)?;

            // stat and rename using utf16
            let file = context.path_utf16(&file_a);
            let stat = context.destack_fs_stat(file)?;
            assert_eq!(stat.size.0, 0);

            let from = context.path_utf16(&file_a);
            let to = context.path_utf16(&file_b);
            context.destack_fs_rename(from, to)?;

            // cleanup
            let to = context.path_utf16(&file_b);
            context.destack_fs_unlink(to)?;

            let dir = context.path_bytes(&temp_dir);
            context.destack_fs_rmdir(dir)?;

            Ok(())
        });
    }
}
