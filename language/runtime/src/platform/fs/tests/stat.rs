#[cfg(windows)]
use super::assert_platform_error_codes_with_privileged_policy;
use super::{temp_dir, with_harness_context};
#[cfg(windows)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(unix, windows))]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{FileMode, OpenFlags, StatxFlags, StatxMask};

/// Poll interval for timestamp-transition assertions.
#[cfg(windows)]
const STAT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
/// Timeout budget for Windows metadata-transition assertions.
#[cfg(windows)]
const STAT_CHANGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Poll one closure until it returns `true` or the timeout expires.
#[cfg(windows)]
fn poll_until(timeout: std::time::Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if predicate() {
            return true;
        }

        std::thread::sleep(STAT_POLL_INTERVAL);
    }

    predicate()
}

/// Synthetic mode tag used for windows symlink metadata assertions.
#[cfg(windows)]
const WINDOWS_S_IFLNK_MODE: u32 = 0o120000;
/// `statx` flag bit for empty-path semantics.
#[cfg(windows)]
const STATX_EMPTY_PATH: u32 = 0x10;
/// `statx` flag bit for dont-sync semantics.
#[cfg(windows)]
const STATX_DONT_SYNC: u32 = 0x4000;
/// One unknown `statx` flag bit for validation tests.
#[cfg(windows)]
const STATX_UNKNOWN_FLAG: u32 = 0x8000;

/// Read file metadata through stat and fstat and compare sizes.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_stat_and_fstat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_stat");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = b"statdata".to_vec();
        let expected_size = payload.len() as u64;
        let payload_value = context.bytes_slice_value(&payload)?;
        context.destack_fs_write(handle, payload_value)?;

        // stat and fstat
        let file = context.path_bytes(&file_path);
        let stat = context.destack_fs_stat(file)?;
        assert_eq!(stat.size.0, expected_size);

        let fstat = context.destack_fs_fstat(handle)?;
        assert_eq!(fstat.size.0, expected_size);

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

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
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // statfs
        let dir = context.path_bytes(&temp_dir);
        let statfs = context.destack_fs_statfs(dir)?;
        assert!(statfs.blocks > 0);
        assert!(statfs.bsize > 0);

        // cleanup
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Read filesystem statistics for one open file handle.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_fstatfs() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_fstatfs");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create file
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        // fstatfs
        let statfs = context.destack_fs_fstatfs(handle)?;
        assert!(statfs.blocks > 0);
        assert!(statfs.bsize > 0);

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Read path metadata through statx fallback lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_statx_returns_file_size() {
    with_harness_context(|mut context| {
        // create one temp directory and one test file
        let temp_dir = temp_dir("fs_statx");
        let file_path = temp_dir.join("file.txt");
        let file_name = context.path_bytes(std::path::Path::new("file.txt"));

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one file with deterministic payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = b"statx-payload".to_vec();
        let expected_size = payload.len() as u64;
        let payload = context.bytes_slice_value(&payload)?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // open the directory and stat the file by relative name
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let statx = context.destack_fs_statx(directory, file_name, StatxFlags(0), StatxMask(0))?;
        assert_eq!(statx.size.0, expected_size);
        assert_ne!(statx.mask.0, 0);

        // cleanup directory resources and paths
        context.destack_fs_closedir(directory)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Read path metadata through the utf16 statx fallback lane on unix hosts.
#[cfg(unix)]
#[test]
fn test_fs_statx_utf16_returns_file_size() {
    with_harness_context(|mut context| {
        // create one temp directory and one test file
        let temp_dir = temp_dir("fs_statx_utf16");
        let file_path = temp_dir.join("file.txt");
        let file_name = context.path_utf16(std::path::Path::new("file.txt"));

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one file with deterministic payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = b"statx-utf16-payload".to_vec();
        let expected_size = payload.len() as u64;
        let payload = context.bytes_slice_value(&payload)?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // open the directory and stat the file by relative utf16 name
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let statx = context.destack_fs_statx(directory, file_name, StatxFlags(0), StatxMask(0))?;
        assert_eq!(statx.size.0, expected_size);
        assert_ne!(statx.mask.0, 0);

        // cleanup directory resources and paths
        context.destack_fs_closedir(directory)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unsupported statx flags on windows fallback lanes.
#[cfg(windows)]
#[test]
fn test_fs_statx_rejects_unsupported_flags_on_windows() {
    with_harness_context(|mut context| {
        // create one temp directory and one test file
        let temp_dir = temp_dir("fs_statx_windows_flags");
        let file_path = temp_dir.join("file.txt");
        let file_name = context.path_bytes(std::path::Path::new("file.txt"));

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one empty file for statx probes
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // open the directory and request one unsupported statx flag
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_statx(
                directory,
                file_name,
                StatxFlags(STATX_DONT_SYNC),
                StatxMask(0),
            ),
            &[PlatformErrorCode::NotSupported],
        )?;

        // cleanup directory resources and paths
        context.destack_fs_closedir(directory)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Allow empty-path statx to target the opened directory handle on windows.
#[cfg(windows)]
#[test]
fn test_fs_statx_empty_path_targets_directory_handle_on_windows() {
    with_harness_context(|mut context| {
        // create one temp directory
        let temp_dir = temp_dir("fs_statx_empty_path_windows");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open the directory and stat it through the empty-path statx mode
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let empty_path = context.path_bytes(std::path::Path::new(""));
        let statx = context.destack_fs_statx(
            directory,
            empty_path,
            StatxFlags(STATX_EMPTY_PATH),
            StatxMask(0),
        )?;

        assert_ne!(statx.mask.0, 0);
        assert_eq!(statx.mode.0 & libc::S_IFMT as u32, libc::S_IFDIR as u32);

        // cleanup
        context.destack_fs_closedir(directory)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unsupported statat flags on windows fallback lanes.
#[cfg(windows)]
#[test]
fn test_fs_statat_rejects_unsupported_flags_on_windows() {
    with_harness_context(|mut context| {
        // create one temp directory and one test file
        let temp_dir = temp_dir("fs_statat_windows_flags");
        let file_path = temp_dir.join("file.txt");
        let file_name = context.path_bytes(std::path::Path::new("file.txt"));

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one empty file for stat probes
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // open the directory and request one unsupported statat flag
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_statat(directory, file_name, crate::platform::fs::AtFlags(0x10)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup directory resources and paths
        context.destack_fs_closedir(directory)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unknown statx flags on windows fallback lanes.
#[cfg(windows)]
#[test]
fn test_fs_statx_rejects_unknown_flags_on_windows() {
    with_harness_context(|mut context| {
        // create one temp directory and one test file
        let temp_dir = temp_dir("fs_statx_unknown_flags_windows");
        let file_path = temp_dir.join("file.txt");
        let file_name = context.path_bytes(std::path::Path::new("file.txt"));

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject unknown statx flags instead of collapsing them into not supported
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_statx(
                directory,
                file_name,
                StatxFlags(STATX_UNKNOWN_FLAG),
                StatxMask(0),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup directory resources and paths
        context.destack_fs_closedir(directory)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Read link metadata without following the symlink target.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_lstat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_lstat");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create target file
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // symlink and lstat
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        #[cfg(unix)]
        context.destack_fs_symlink(target, link, SymlinkType::File)?;

        #[cfg(windows)]
        if let Err(error) = context.destack_fs_symlink(target, link, SymlinkType::File) {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[PlatformErrorCode::IoPermissionDenied],
            )?;

            let target = context.path_bytes(&file_path);
            context.destack_fs_unlink(target)?;
            let dir = context.path_bytes(&temp_dir);
            context.destack_fs_rmdir(dir)?;

            return Ok(());
        }

        let link = context.path_bytes(&link_path);
        let stat = context.destack_fs_lstat(link)?;
        #[cfg(unix)]
        assert_eq!(stat.mode.0 & libc::S_IFMT as u32, libc::S_IFLNK as u32);
        #[cfg(windows)]
        assert_eq!(stat.mode.0 & libc::S_IFMT as u32, WINDOWS_S_IFLNK_MODE);

        // cleanup
        let link = context.path_bytes(&link_path);
        context.destack_fs_unlink(link)?;
        let target = context.path_bytes(&file_path);
        context.destack_fs_unlink(target)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Preserve distinct change time and birth time semantics on windows stat fallback lanes.
#[cfg(windows)]
#[test]
fn test_fs_stat_windows_reports_change_time_distinct_from_birth_time() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_stat_windows_ctime");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create the file and capture its initial metadata
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let file = context.path_bytes(&file_path);
        let initial = context.destack_fs_stat(file)?;

        // change one attribute and require ctime to advance independently of birth time
        let file = context.path_bytes(&file_path);
        context.destack_fs_chmod(file, FileMode(0o444))?;

        // poll until the filesystem reports the metadata transition or the timeout expires
        let mut updated = None;
        let changed = poll_until(STAT_CHANGE_TIMEOUT, || {
            let file = context.path_bytes(&file_path);
            let stat = match context.destack_fs_stat(file) {
                Ok(stat) => stat,
                Err(_) => return false,
            };
            let did_change = stat.ctime_ns > initial.ctime_ns;
            updated = Some(stat);
            did_change
        });
        let updated = updated.expect("polled stat result should exist");

        assert_eq!(updated.birthtime_ns, initial.birthtime_ns);
        assert!(changed, "expected ctime to advance after chmod");
        assert!(updated.ctime_ns >= initial.ctime_ns);
        assert_ne!(updated.ctime_ns, updated.birthtime_ns);

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
