#[cfg(windows)]
mod windows_tests {
    use super::super::{FsHarness, path_bytes, path_utf16, temp_dir, with_native_harness};
    use crate::platform::fs::{
        FileMode, OpenFlags, Stat, destack_fs_close, destack_fs_mkdir_bytes, destack_fs_open_utf16,
        destack_fs_rename_utf16, destack_fs_rmdir_bytes, destack_fs_stat_utf16,
        destack_fs_unlink_utf16,
    };
    use crate::platform::resource::{FileHandle, ResourceId};

    #[test]
    fn test_fs_utf16_open_rename_unlink() {
        with_native_harness(|harness| {
            let runtime = harness.runtime();
            let temp_dir = temp_dir("fs_utf16");
            let file_a = temp_dir.join("alpha.txt");
            let file_b = temp_dir.join("beta.txt");

            harness.with_context(|| {
                let (_bytes, dir) = path_bytes(&temp_dir);
                let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
                runtime.assert_status_ok(status, "mkdir");

                let (_wide, file) = path_utf16(&file_a);
                let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
                let mut handle = FileHandle(ResourceId(0));
                let status =
                    unsafe { destack_fs_open_utf16(&mut handle, file, flags, FileMode(0o644)) };
                runtime.assert_status_ok(status, "open utf16");
                let status = unsafe { destack_fs_close(handle) };
                runtime.assert_status_ok(status, "close");

                let mut stat = std::mem::MaybeUninit::<Stat>::uninit();
                let status = unsafe { destack_fs_stat_utf16(stat.as_mut_ptr(), file) };
                runtime.assert_status_ok(status, "stat utf16");
                let stat = unsafe { stat.assume_init() };
                assert_eq!(stat.size.0, 0);

                let (_wide, from) = path_utf16(&file_a);
                let (_wide, to) = path_utf16(&file_b);
                let status = unsafe { destack_fs_rename_utf16(from, to) };
                runtime.assert_status_ok(status, "rename utf16");

                let (_wide, to) = path_utf16(&file_b);
                let status = unsafe { destack_fs_unlink_utf16(to) };
                runtime.assert_status_ok(status, "unlink utf16");

                let (_bytes, dir) = path_bytes(&temp_dir);
                let status = unsafe { destack_fs_rmdir_bytes(dir) };
                runtime.assert_status_ok(status, "rmdir");
            });
        });
    }
}
