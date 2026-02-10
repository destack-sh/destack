use super::{FsHarness, native_slice, native_slice_mut, path_bytes, temp_dir, with_native_harness};
#[cfg(unix)]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{
    FileMode, FileOffset, OpenFlags, PathBytes, destack_fs_close, destack_fs_copyfile_bytes,
    destack_fs_link_bytes, destack_fs_mkdir_bytes, destack_fs_open_bytes, destack_fs_read,
    destack_fs_readlink_bytes, destack_fs_rename_bytes, destack_fs_rmdir_bytes,
    destack_fs_symlink_bytes, destack_fs_unlink_bytes, destack_fs_write,
};
use crate::platform::resource::{FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_rename_unlink_copyfile() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_path");
        let file_a = temp_dir.join("a.txt");
        let file_b = temp_dir.join("b.txt");
        let file_c = temp_dir.join("c.txt");
        let file_d = temp_dir.join("d.txt");

        harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, path) = path_bytes(&file_a);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, path, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            let payload = b"destack".to_vec();
            let slice = native_slice(&payload);
            let mut out = 0u64;
            let status = unsafe { destack_fs_write(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "write");
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close");

            let (_bytes, from) = path_bytes(&file_a);
            let (_bytes, to) = path_bytes(&file_b);
            let status = unsafe { destack_fs_rename_bytes(from, to) };
            runtime.assert_status_ok(status, "rename");

            let (_bytes, from) = path_bytes(&file_b);
            let (_bytes, to) = path_bytes(&file_c);
            let status = unsafe { destack_fs_link_bytes(from, to) };
            runtime.assert_status_ok(status, "link");

            let (_bytes, from) = path_bytes(&file_b);
            let (_bytes, to) = path_bytes(&file_d);
            let status = unsafe { destack_fs_copyfile_bytes(from, to, 0) };
            runtime.assert_status_ok(status, "copyfile");

            let (_bytes, path) = path_bytes(&file_c);
            let flags = OpenFlags(libc::O_RDONLY as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, path, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open link");
            let mut buffer = vec![0u8; 16];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_fs_read(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "read link");
            buffer.truncate(out as usize);
            assert_eq!(buffer, b"destack");
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close link");

            let (_bytes, path) = path_bytes(&file_d);
            let flags = OpenFlags(libc::O_RDONLY as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, path, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open copy");
            let mut buffer = vec![0u8; 16];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_fs_read(&mut out, handle, slice, FileOffset(0)) };
            runtime.assert_status_ok(status, "read copy");
            buffer.truncate(out as usize);
            assert_eq!(buffer, b"destack");
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close copy");

            let (_bytes, to) = path_bytes(&file_b);
            let status = unsafe { destack_fs_unlink_bytes(to) };
            runtime.assert_status_ok(status, "unlink b");

            let (_bytes, to) = path_bytes(&file_c);
            let status = unsafe { destack_fs_unlink_bytes(to) };
            runtime.assert_status_ok(status, "unlink c");

            let (_bytes, to) = path_bytes(&file_d);
            let status = unsafe { destack_fs_unlink_bytes(to) };
            runtime.assert_status_ok(status, "unlink d");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}

#[cfg(unix)]
#[test]
fn test_fs_symlink_readlink() {
    with_native_harness(|harness| {
        // runtime and temp directory
        let runtime = harness.runtime();
        let temp_dir = temp_dir("fs_symlink");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        harness.with_context(|| {
            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_mkdir_bytes(dir, FileMode(0o755)) };
            runtime.assert_status_ok(status, "mkdir");

            let (_bytes, path) = path_bytes(&file_path);
            let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
            let mut handle = FileHandle(ResourceId(0));
            let status =
                unsafe { destack_fs_open_bytes(&mut handle, path, flags, FileMode(0o644)) };
            runtime.assert_status_ok(status, "open");
            let status = unsafe { destack_fs_close(handle) };
            runtime.assert_status_ok(status, "close");

            let (_bytes, target) = path_bytes(&file_path);
            let (_bytes, link) = path_bytes(&link_path);
            let status = unsafe { destack_fs_symlink_bytes(target, link, SymlinkType::File) };
            runtime.assert_status_ok(status, "symlink");

            let mut out = std::mem::MaybeUninit::<PathBytes>::uninit();
            let status = unsafe { destack_fs_readlink_bytes(out.as_mut_ptr(), link) };
            runtime.assert_status_ok(status, "readlink");
            let path = unsafe { out.assume_init() };
            let bytes = unsafe { path.0.as_slice() }.expect("readlink bytes should be valid");
            assert!(bytes.ends_with(b"file.txt"));

            let (_bytes, link) = path_bytes(&link_path);
            let status = unsafe { destack_fs_unlink_bytes(link) };
            runtime.assert_status_ok(status, "unlink link");

            let (_bytes, path) = path_bytes(&file_path);
            let status = unsafe { destack_fs_unlink_bytes(path) };
            runtime.assert_status_ok(status, "unlink file");

            let (_bytes, dir) = path_bytes(&temp_dir);
            let status = unsafe { destack_fs_rmdir_bytes(dir) };
            runtime.assert_status_ok(status, "rmdir");
        });
    });
}
