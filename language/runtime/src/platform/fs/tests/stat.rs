use super::{temp_dir, with_harness_context};
#[cfg(unix)]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{FileMode, OpenFlags};

/// Read file metadata through stat and fstat and compare sizes.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_stat_and_fstat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_stat");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        let payload = b"statdata".to_vec();
        context.write(handle, &payload)?;
        let expected_size = payload.len() as u64;

        // stat and fstat
        let file = context.path_bytes(&file_path);
        let stat = context.stat(file)?;
        assert_eq!(stat.size.0, expected_size);

        let fstat = context.fstat(handle)?;
        assert_eq!(fstat.size.0, expected_size);

        // cleanup
        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

/// Read filesystem statistics for one directory path.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_statfs() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_statfs");
        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // statfs
        let dir = context.path_bytes(&temp_dir);
        let statfs = context.statfs(dir)?;
        assert!(statfs.blocks > 0);
        assert!(statfs.bsize > 0);

        // cleanup
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

/// Read filesystem statistics for one open file handle.
#[cfg(unix)]
#[test]
fn test_fs_fstatfs() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_fstatfs");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create file
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;

        // fstatfs
        let statfs = context.fstatfs(handle)?;
        assert!(statfs.blocks > 0);
        assert!(statfs.bsize > 0);

        // cleanup
        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

/// Read link metadata without following the symlink target.
#[cfg(unix)]
#[test]
fn test_fs_lstat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_lstat");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create target file
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        // symlink and lstat
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.symlink(target, link, SymlinkType::File)?;

        let link = context.path_bytes(&link_path);
        let _stat = context.lstat(link)?;

        // cleanup
        let link = context.path_bytes(&link_path);
        context.unlink(link)?;
        let target = context.path_bytes(&file_path);
        context.unlink(target)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
