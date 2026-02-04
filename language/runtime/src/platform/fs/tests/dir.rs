use super::{FsHarness, native_array, path_bytes, temp_dir, with_native_harness};
use crate::platform::NativeArray;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    Dirent, DirentKind, FileMode, PathBytesAbi, destack_fs_closedir, destack_fs_mkdir_bytes,
    destack_fs_mkdirat_bytes, destack_fs_opendir_bytes, destack_fs_readdir, destack_fs_rmdir_bytes,
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
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, child) = path_bytes(&child_dir);
            let status = unsafe { destack_fs_mkdir_bytes(child, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir child");

            let mut handle = DirectoryHandle(ResourceId(0));
            let status = unsafe { destack_fs_opendir_bytes(&mut handle, dir) };
            runtime.assert_status_ok(status, "opendir");
            handle
        });

        // read directory entries
        let entries = harness.with_context(|| {
            let mut out = std::mem::MaybeUninit::<NativeArray<Dirent>>::uninit();
            let status = unsafe { destack_fs_readdir(out.as_mut_ptr(), handle) };
            runtime.assert_status_ok(status, "readdir");
            let array = unsafe { out.assume_init() };
            let slice = unsafe { array.as_slice() }.expect("dirent slice should be valid");
            slice
                .iter()
                .map(|entry| {
                    let name =
                        unsafe { entry.name.as_str() }.expect("dirent name should be valid utf8");
                    (name.to_string(), entry.kind)
                })
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
            let status = unsafe { destack_fs_closedir(handle) };
            runtime.assert_status_ok(status, "closedir");

            let (_bytes, child) = path_bytes(&child_dir);
            let status = unsafe { destack_fs_rmdir_bytes(child) };
            runtime.assert_status_ok(status, "rmdir child");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
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
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let mut handle = DirectoryHandle(ResourceId(0));
            let status = unsafe { destack_fs_opendir_bytes(&mut handle, dir) };
            runtime.assert_status_ok(status, "opendir");

            let mut name = b"child".to_vec();
            let path = PathBytesAbi::<NativeAbi>(native_array(&mut name));
            let status = unsafe { destack_fs_mkdirat_bytes(handle, path, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdirat");

            let status = unsafe { destack_fs_closedir(handle) };
            runtime.assert_status_ok(status, "closedir");

            let (_bytes, child) = path_bytes(&child_dir);
            let status = unsafe { destack_fs_rmdir_bytes(child) };
            runtime.assert_status_ok(status, "rmdir child");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
