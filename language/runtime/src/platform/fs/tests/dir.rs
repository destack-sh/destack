use super::{FsHarness, os_path_string, path_bytes, temp_dir, with_native_harness};
use crate::platform::NativeArray;
use crate::platform::fs::{
    Dirent, DirentKind, FileMode, destack_fs_dir_closedir, destack_fs_dir_mkdir,
    destack_fs_dir_mkdirat, destack_fs_dir_opendir, destack_fs_dir_readdir, destack_fs_dir_rmdir,
};
use crate::platform::resource::{DirectoryHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdir_opendir_readdir_closedir() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_dir");
        let child_dir = temp_dir.join("child");

        // create directory and subdir
        let handle = harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_mkdir(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, child) = path_bytes(&child_dir);
            let status = unsafe { destack_fs_dir_mkdir(child, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir child");

            let mut handle = DirectoryHandle(ResourceId(0));
            let status = unsafe { destack_fs_dir_opendir(&mut handle, dir) };
            runtime.assert_status_ok(status, "opendir");
            handle
        });

        // read directory entries
        let entries = harness.with_context(|| {
            let mut out = std::mem::MaybeUninit::<NativeArray<Dirent>>::uninit();
            let status = unsafe { destack_fs_dir_readdir(out.as_mut_ptr(), handle) };
            runtime.assert_status_ok(status, "readdir");
            let array = unsafe { out.assume_init() };
            let slice = unsafe { array.as_slice() }.expect("dirent slice should be valid");
            slice
                .iter()
                .map(|entry| (os_path_string(&entry.name), entry.kind))
                .collect::<Vec<_>>()
        });

        assert!(
            entries
                .iter()
                .any(|(name, kind)| name == "child" && *kind == DirentKind::Directory)
        );
        assert!(!entries.iter().any(|(name, _)| name == "."));
        assert!(!entries.iter().any(|(name, _)| name == ".."));

        // close directory and cleanup
        harness.with_context(|| {
            let status = unsafe { destack_fs_dir_closedir(handle) };
            runtime.assert_status_ok(status, "closedir");

            let (_bytes, child) = path_bytes(&child_dir);
            let status = unsafe { destack_fs_dir_rmdir(child) };
            runtime.assert_status_ok(status, "rmdir child");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_rmdir(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdirat_bytes() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_dirat");
        let child_dir = temp_dir.join("child");

        harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_mkdir(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let mut handle = DirectoryHandle(ResourceId(0));
            let status = unsafe { destack_fs_dir_opendir(&mut handle, dir) };
            runtime.assert_status_ok(status, "opendir");

            let name_path = std::path::Path::new("child");
            let (_name_bytes, path) = path_bytes(name_path);
            let status = unsafe { destack_fs_dir_mkdirat(handle, path, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdirat");

            let status = unsafe { destack_fs_dir_closedir(handle) };
            runtime.assert_status_ok(status, "closedir");

            let (_bytes, child) = path_bytes(&child_dir);
            let status = unsafe { destack_fs_dir_rmdir(child) };
            runtime.assert_status_ok(status, "rmdir child");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_dir_rmdir(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
