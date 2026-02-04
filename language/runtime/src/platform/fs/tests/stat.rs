use super::{FsHarness, path_bytes, temp_dir, with_native_harness};
use crate::platform::fs::{
    FileMode, FileOffset, OpenFlags, Stat, StatFs, SymlinkType, destack_fs_close, destack_fs_fstat,
    destack_fs_lstat_bytes, destack_fs_mkdir_bytes, destack_fs_open_bytes, destack_fs_rmdir_bytes,
    destack_fs_stat_bytes, destack_fs_statfs_bytes, destack_fs_symlink_bytes,
    destack_fs_unlink_bytes, destack_fs_write,
};
use crate::platform::resource::{FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_stat_and_fstat() {
    with_native_harness(|harness| {
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_stat");
        let file_path = temp_dir.join("file.txt");

        let (handle, expected_size) = harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            let payload = b"statdata".to_vec();
            let slice = super::native_slice(&payload);
            let mut out = 0u64;
            let status = unsafe { destack_fs_write(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "write");
            (handle, payload.len() as u64)
        });

        harness.with_context(|| {
            let (_bytes, file) = path_bytes(&file_path);
            let mut stat = std::mem::MaybeUninit::<Stat>::uninit();
            let status = unsafe { destack_fs_stat_bytes(stat.as_mut_ptr(), file) };
            runtime.assert_status_ok(status, "stat");
            let stat = unsafe { stat.assume_init() };
            assert_eq!(stat.size.0, expected_size);

            let mut fstat = std::mem::MaybeUninit::<Stat>::uninit();
            let status = unsafe { destack_fs_fstat(fstat.as_mut_ptr(), handle) };
            runtime.assert_status_ok(status, "fstat");
            let fstat = unsafe { fstat.assume_init() };
            assert_eq!(fstat.size.0, expected_size);
        });

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

#[cfg(any(unix, windows))]
#[test]
fn test_fs_statfs() {
    with_native_harness(|harness| {
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_statfs");

        harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let mut statfs = std::mem::MaybeUninit::<StatFs>::uninit();
            let status = unsafe { destack_fs_statfs_bytes(statfs.as_mut_ptr(), dir) };
            runtime.assert_status_ok(status, "statfs");
            let statfs = unsafe { statfs.assume_init() };
            assert!(statfs.blocks > 0);
            assert!(statfs.bsize > 0);

            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}

#[cfg(unix)]
#[test]
fn test_fs_lstat() {
    with_native_harness(|harness| {
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_lstat");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, file) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, file, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close");

            let (_bytes, target) = path_bytes(&file_path);
            let (_bytes, link) = path_bytes(&link_path);
            let status = unsafe { destack_fs_symlink_bytes(target, link, SymlinkType::File) };
            runtime.assert_status_ok(status, "symlink");

            let mut stat = std::mem::MaybeUninit::<Stat>::uninit();
            let status = unsafe { destack_fs_lstat_bytes(stat.as_mut_ptr(), link) };
            runtime.assert_status_ok(status, "lstat");

            let status = unsafe { destack_fs_unlink_bytes(link) };
            runtime.assert_status_ok(status, "unlink link");
            let status = unsafe { destack_fs_unlink_bytes(target) };
            runtime.assert_status_ok(status, "unlink file");
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
