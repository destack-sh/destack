#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{FileMode, OpenFlags, SymlinkType, XattrFlags};

/// Roundtrip extended attributes through path and file-descriptor apis.
#[cfg(unix)]
#[test]
fn test_fs_xattr_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let name = "user.destack";
        let value = b"xattr";
        let allowed = [PlatformErrorCode::NotSupported];

        let path = context.path_bytes(&file_path);
        let set_result = context.destack_fs_setxattr(
            path,
            context.string_value(name),
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        );
        let wrote = context.result_ok_or_codes(set_result, "setxattr", &allowed)?;

        // when supported, path xattr reads and lists should reflect writes
        if wrote.is_some() {
            let path = context.path_bytes(&file_path);
            let read = context.destack_fs_getxattr(path, context.string_value(name))?;
            let read = context.bytes_from_array_value(read)?;
            assert_eq!(read, value);

            let list = context.destack_fs_listxattr(path)?;
            let list = context.string_list_from_value(list)?;
            assert!(list.iter().any(|entry| entry == name));

            context.destack_fs_removexattr(path, context.string_value(name))?;
        }

        let file = context.path_bytes(&file_path);
        let handle =
            context.destack_fs_open(file, OpenFlags(libc::O_RDONLY as u32), FileMode(0o644))?;
        let set_result = context.destack_fs_fsetxattr(
            handle,
            context.string_value(name),
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        );
        let wrote = context.result_ok_or_codes(set_result, "fsetxattr", &allowed)?;

        // when supported, fd xattr reads and lists should reflect writes
        if wrote.is_some() {
            let read = context.destack_fs_fgetxattr(handle, context.string_value(name))?;
            let read = context.bytes_from_array_value(read)?;
            assert_eq!(read, value);

            let list = context.destack_fs_flistxattr(handle)?;
            let list = context.string_list_from_value(list)?;
            assert!(list.iter().any(|entry| entry == name));

            context.destack_fs_fremovexattr(handle, context.string_value(name))?;
        }

        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Roundtrip extended attributes on symlink paths.
#[cfg(unix)]
#[test]
fn test_fs_xattr_symlink() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_symlink");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.destack_fs_symlink(target, link, SymlinkType::File)?;

        let name = "user.destack.link";
        let value = b"link";
        let allowed = [PlatformErrorCode::NotSupported];
        let link = context.path_bytes(&link_path);
        let set_result = context.destack_fs_lsetxattr(
            link,
            context.string_value(name),
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        );
        let wrote = context.result_ok_or_codes(set_result, "lsetxattr", &allowed)?;

        // when supported, symlink xattr reads and lists should reflect writes
        if wrote.is_some() {
            let link = context.path_bytes(&link_path);
            let read = context.destack_fs_lgetxattr(link, context.string_value(name))?;
            let read = context.bytes_from_array_value(read)?;
            assert_eq!(read, value);

            let link = context.path_bytes(&link_path);
            let list = context.destack_fs_llistxattr(link)?;
            let list = context.string_list_from_value(list)?;
            assert!(list.iter().any(|entry| entry == name));

            let link = context.path_bytes(&link_path);
            context.destack_fs_lremovexattr(link, context.string_value(name))?;
        }

        let link = context.path_bytes(&link_path);
        context.destack_fs_unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
