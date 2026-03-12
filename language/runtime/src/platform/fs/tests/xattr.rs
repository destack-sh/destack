#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{assert_platform_error_codes_with_privileged_policy, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{FileMode, OpenFlags, SymlinkType, XattrFlags};

/// Roundtrip extended attributes through path and file-descriptor apis.
#[cfg(any(unix, windows))]
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

        let path = context.path_bytes(&file_path);
        context.destack_fs_setxattr(
            path,
            context.string_value(name),
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        )?;

        // path xattr reads and lists should reflect writes
        let path = context.path_bytes(&file_path);
        let read = context.destack_fs_getxattr(path, context.string_value(name))?;
        let read = context.bytes_from_array_value(read)?;
        assert_eq!(read, value);

        let list = context.destack_fs_listxattr(path)?;
        let list = context.string_list_from_value(list)?;
        assert!(list.iter().any(|entry| entry == name));

        context.destack_fs_removexattr(path, context.string_value(name))?;

        let file = context.path_bytes(&file_path);
        let handle =
            context.destack_fs_open(file, OpenFlags(libc::O_RDONLY as u32), FileMode(0o644))?;
        context.destack_fs_fsetxattr(
            handle,
            context.string_value(name),
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        )?;

        // fd xattr reads and lists should reflect writes
        let read = context.destack_fs_fgetxattr(handle, context.string_value(name))?;
        let read = context.bytes_from_array_value(read)?;
        assert_eq!(read, value);

        let list = context.destack_fs_flistxattr(handle)?;
        let list = context.string_list_from_value(list)?;
        assert!(list.iter().any(|entry| entry == name));

        context.destack_fs_fremovexattr(handle, context.string_value(name))?;

        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Roundtrip extended attributes through raw-name byte APIs.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_xattr_bytes_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_bytes");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let name = b"user.destack.bytes";
        let value = b"bytes-lane";

        let path = context.path_bytes(&file_path);
        context.destack_fs_setxattr_bytes(
            path,
            context.bytes_slice_value(name)?,
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        )?;

        // path xattr byte reads and lists should reflect writes
        let path = context.path_bytes(&file_path);
        let read = context.destack_fs_getxattr_bytes(path, context.bytes_slice_value(name)?)?;
        let read = context.bytes_from_array_value(read)?;
        assert_eq!(read, value);

        let path = context.path_bytes(&file_path);
        let list = context.destack_fs_listxattr_bytes(path)?;
        let list = context.bytes_array_list_from_value(list)?;
        assert!(list.iter().any(|entry| entry == name));

        let path = context.path_bytes(&file_path);
        context.destack_fs_removexattr_bytes(path, context.bytes_slice_value(name)?)?;

        let file = context.path_bytes(&file_path);
        let handle =
            context.destack_fs_open(file, OpenFlags(libc::O_RDONLY as u32), FileMode(0o644))?;
        context.destack_fs_fsetxattr_bytes(
            handle,
            context.bytes_slice_value(name)?,
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        )?;

        // fd xattr byte reads and lists should reflect writes
        let read = context.destack_fs_fgetxattr_bytes(handle, context.bytes_slice_value(name)?)?;
        let read = context.bytes_from_array_value(read)?;
        assert_eq!(read, value);

        let list = context.destack_fs_flistxattr_bytes(handle)?;
        let list = context.bytes_array_list_from_value(list)?;
        assert!(list.iter().any(|entry| entry == name));

        context.destack_fs_fremovexattr_bytes(handle, context.bytes_slice_value(name)?)?;

        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unknown xattr flag bits before host syscall dispatch.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_xattr_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_unknown_flags");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject unknown bits on the public path lane
        let path = context.path_bytes(&file_path);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_setxattr(
                path,
                context.string_value("user.destack.flags"),
                context.bytes_slice_value(b"value")?,
                XattrFlags(0x4),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject conflicting xattr create and replace flags before host syscall dispatch.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_xattr_rejects_conflicting_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_conflicting_flags");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject mutually exclusive create and replace bits
        let path = context.path_bytes(&file_path);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_setxattr(
                path,
                context.string_value("user.destack.flags"),
                context.bytes_slice_value(b"value")?,
                XattrFlags(0x3),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Roundtrip large extended-attribute payloads and grow the backend query buffers as needed.
#[cfg(windows)]
#[test]
fn test_fs_xattr_large_value_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_large_value");
        let file_path = temp_dir.join("file.txt");
        let value = vec![0x5au8; 8192];

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let name = "user.destack.large";
        let path = context.path_bytes(&file_path);
        context.destack_fs_setxattr(
            path,
            context.string_value(name),
            context.bytes_slice_value(&value)?,
            XattrFlags(0),
        )?;

        // large values should roundtrip through query growth
        let path = context.path_bytes(&file_path);
        let read = context.destack_fs_getxattr(path, context.string_value(name))?;
        let read = context.bytes_from_array_value(read)?;
        assert_eq!(read, value);

        let path = context.path_bytes(&file_path);
        let list = context.destack_fs_listxattr(path)?;
        let list = context.string_list_from_value(list)?;
        assert!(list.iter().any(|entry| entry == name));

        let path = context.path_bytes(&file_path);
        context.destack_fs_removexattr(path, context.string_value(name))?;

        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Roundtrip extended attributes with non-UTF8 raw names when the host permits them.
#[cfg(unix)]
#[test]
fn test_fs_xattr_bytes_non_utf8_name_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_bytes_non_utf8");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let name = b"user.destack.\xffbytes";
        let value = b"non-utf8-name";
        let allowed = [
            PlatformErrorCode::NotSupported,
            PlatformErrorCode::IoInvalidData,
        ];

        let path = context.path_bytes(&file_path);
        let set_result = context.destack_fs_setxattr_bytes(
            path,
            context.bytes_slice_value(name)?,
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        );
        let wrote = context.result_ok_or_codes(set_result, "setxattrBytes(non-utf8)", &allowed)?;

        // when supported, non-utf8 names should roundtrip as raw bytes
        if wrote.is_some() {
            let path = context.path_bytes(&file_path);
            let read = context.destack_fs_getxattr_bytes(path, context.bytes_slice_value(name)?)?;
            let read = context.bytes_from_array_value(read)?;
            assert_eq!(read, value);

            let path = context.path_bytes(&file_path);
            let list = context.destack_fs_listxattr_bytes(path)?;
            let list = context.bytes_array_list_from_value(list)?;
            assert!(list.iter().any(|entry| entry == name));

            let path = context.path_bytes(&file_path);
            context.destack_fs_removexattr_bytes(path, context.bytes_slice_value(name)?)?;
        }

        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject raw xattr names with embedded NUL bytes before host syscall dispatch.
#[cfg(unix)]
#[test]
fn test_fs_xattr_bytes_rejects_nul_name() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_bytes_nul_name");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let path = context.path_bytes(&file_path);
        let result = context.destack_fs_setxattr_bytes(
            path,
            context.bytes_slice_value(b"user.destack\0nul")?,
            context.bytes_slice_value(b"value")?,
            XattrFlags(0),
        );
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

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

/// Roundtrip extended attributes with raw names on symlink paths.
#[cfg(unix)]
#[test]
fn test_fs_xattr_symlink_bytes() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_symlink_bytes");
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

        let name = b"user.destack.link.bytes";
        let value = b"link-bytes";
        let allowed = [PlatformErrorCode::NotSupported];
        let link = context.path_bytes(&link_path);
        let set_result = context.destack_fs_lsetxattr_bytes(
            link,
            context.bytes_slice_value(name)?,
            context.bytes_slice_value(value)?,
            XattrFlags(0),
        );
        let wrote = context.result_ok_or_codes(set_result, "lsetxattrBytes", &allowed)?;

        // when supported, symlink xattr byte reads and lists should reflect writes
        if wrote.is_some() {
            let link = context.path_bytes(&link_path);
            let read =
                context.destack_fs_lgetxattr_bytes(link, context.bytes_slice_value(name)?)?;
            let read = context.bytes_from_array_value(read)?;
            assert_eq!(read, value);

            let link = context.path_bytes(&link_path);
            let list = context.destack_fs_llistxattr_bytes(link)?;
            let list = context.bytes_array_list_from_value(list)?;
            assert!(list.iter().any(|entry| entry == name));

            let link = context.path_bytes(&link_path);
            context.destack_fs_lremovexattr_bytes(link, context.bytes_slice_value(name)?)?;
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
