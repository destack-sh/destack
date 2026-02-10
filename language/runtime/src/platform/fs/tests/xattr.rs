#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{FileMode, OpenFlags, XattrFlags};

#[cfg(unix)]
#[test]
fn test_fs_xattr_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        let name = "user.destack";
        let value = b"xattr";
        let allowed = [PlatformErrorCode::NotSupported];

        let path = context.path_bytes(&file_path);
        let set_result = context.setxattr(path.clone(), name, value, XattrFlags(0));
        let wrote = context.result_ok_or_codes(set_result, "setxattr", &allowed)?;

        if wrote.is_some() {
            let path = context.path_bytes(&file_path);
            let read = context.getxattr(path.clone(), name)?;
            assert_eq!(read, value);

            let list = context.listxattr(path.clone())?;
            assert!(list.iter().any(|entry| entry == name));

            context.removexattr(path, name)?;
        }

        let file = context.path_bytes(&file_path);
        let handle = context.open(file, OpenFlags(libc::O_RDONLY as u32), FileMode(0o644))?;
        let set_result = context.fsetxattr(handle, name, value, XattrFlags(0));
        let wrote = context.result_ok_or_codes(set_result, "fsetxattr", &allowed)?;

        if wrote.is_some() {
            let read = context.fgetxattr(handle, name)?;
            assert_eq!(read, value);

            let list = context.flistxattr(handle)?;
            assert!(list.iter().any(|entry| entry == name));

            context.fremovexattr(handle, name)?;
        }

        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_fs_xattr_symlink() {
    use crate::platform::fs::SymlinkType;

    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_xattr_symlink");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.symlink(target, link, SymlinkType::File)?;

        let name = "user.destack.link";
        let value = b"link";
        let allowed = [PlatformErrorCode::NotSupported];
        let link = context.path_bytes(&link_path);
        let set_result = context.lsetxattr(link.clone(), name, value, XattrFlags(0));
        let wrote = context.result_ok_or_codes(set_result, "lsetxattr", &allowed)?;

        if wrote.is_some() {
            let link = context.path_bytes(&link_path);
            let read = context.lgetxattr(link, name)?;
            assert_eq!(read, value);

            let link = context.path_bytes(&link_path);
            let list = context.llistxattr(link)?;
            assert!(list.iter().any(|entry| entry == name));

            let link = context.path_bytes(&link_path);
            context.lremovexattr(link, name)?;
        }

        let link = context.path_bytes(&link_path);
        context.unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
