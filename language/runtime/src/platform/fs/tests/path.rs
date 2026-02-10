use super::{temp_dir, with_harness_context};
#[cfg(unix)]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{CopyFlags, FileMode, OpenFlags};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_rename_unlink_copyfile() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_path");
        let file_a = temp_dir.join("a.txt");
        let file_b = temp_dir.join("b.txt");
        let file_c = temp_dir.join("c.txt");
        let file_d = temp_dir.join("d.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // write initial file
        let path = context.path_bytes(&file_a);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(path, flags, FileMode(0o644))?;
        let payload = b"destack".to_vec();
        context.write(handle, &payload)?;
        context.close(handle)?;

        // rename, link, and copy
        let from = context.path_bytes(&file_a);
        let to = context.path_bytes(&file_b);
        context.rename(from, to)?;

        let from = context.path_bytes(&file_b);
        let to = context.path_bytes(&file_c);
        context.link(from, to)?;

        let from = context.path_bytes(&file_b);
        let to = context.path_bytes(&file_d);
        context.copyfile(from, to, CopyFlags(0))?;

        let path = context.path_bytes(&file_c);
        let flags = OpenFlags(libc::O_RDONLY as u32);
        let handle = context.open(path, flags, FileMode(0o644))?;
        let mut buffer = vec![0u8; 16];
        let out = context.read(handle, &mut buffer)?;
        buffer.truncate(out as usize);
        assert_eq!(buffer, b"destack");
        context.close(handle)?;

        let path = context.path_bytes(&file_d);
        let handle = context.open(path, flags, FileMode(0o644))?;
        let mut buffer = vec![0u8; 16];
        let out = context.read(handle, &mut buffer)?;
        buffer.truncate(out as usize);
        assert_eq!(buffer, b"destack");
        context.close(handle)?;

        // cleanup
        let to = context.path_bytes(&file_b);
        context.unlink(to)?;
        let to = context.path_bytes(&file_c);
        context.unlink(to)?;
        let to = context.path_bytes(&file_d);
        context.unlink(to)?;

        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_fs_symlink_readlink() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_symlink");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create target file
        let path = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(path, flags, FileMode(0o644))?;
        context.close(handle)?;

        // symlink and readlink
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.symlink(target, link, SymlinkType::File)?;

        let link = context.path_bytes(&link_path);
        let path = context.readlink(link)?;
        let bytes = context.path_ref_bytes(path);
        assert!(bytes.ends_with(b"file.txt"));

        // cleanup
        let link = context.path_bytes(&link_path);
        context.unlink(link)?;

        let path = context.path_bytes(&file_path);
        context.unlink(path)?;

        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_realpath() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_realpath");
        let file_path = temp_dir.join("real.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create file
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        // resolve real path
        let file = context.path_bytes(&file_path);
        let resolved = context.realpath(file)?;
        let name = context.path_ref_string(resolved);
        assert!(name.ends_with("real.txt"));

        // cleanup
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
#[test]
fn test_fs_non_utf8_realpath_and_readlink_bytes() {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_non_utf8");
        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create a file with a non-utf8 byte in the name
        let file_name = OsString::from_vec(b"target_\xff.bin".to_vec());
        let file_path = temp_dir.join(file_name);
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        // resolve realpath and ensure the non-utf8 byte survives
        let file = context.path_bytes(&file_path);
        let resolved = context.realpath(file)?;
        let resolved = context.path_ref_bytes(resolved);
        assert!(resolved.contains(&0xff));

        // create a symlink and verify readlink bytes preserve the target bytes
        let link_name = OsString::from_vec(b"link_\xfe.bin".to_vec());
        let link_path = temp_dir.join(link_name);
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.symlink(target, link, SymlinkType::File)?;

        let link = context.path_bytes(&link_path);
        let linked_target = context.readlink(link)?;
        let linked_target = context.path_ref_bytes(linked_target);
        let expected = file_path.as_os_str().as_bytes();
        assert_eq!(linked_target.as_slice(), expected);

        // cleanup
        let link = context.path_bytes(&link_path);
        context.unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_fs_utf16_input_path_on_unix_uses_utf8_bytes() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_utf16_unix");
        let file_path = temp_dir.join("utf8_name.txt");

        // create directory
        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create file through utf16 path encoding
        let file = context.path_utf16(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.write(handle, b"payload")?;
        context.close(handle)?;

        // stat and resolve through utf16 path encoding
        let file = context.path_utf16(&file_path);
        let stat = context.stat(file)?;
        assert_eq!(stat.size.0, 7);

        let file = context.path_utf16(&file_path);
        let resolved = context.realpath(file)?;
        let resolved = context.path_ref_string(resolved);
        assert!(resolved.ends_with("utf8_name.txt"));

        // cleanup
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
#[test]
fn test_fs_utf16_output_path_requires_utf8_on_unix() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_utf16_output_utf8");
        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create a file with non-utf8 bytes in the name
        let file_name = OsString::from_vec(b"target_\xff.bin".to_vec());
        let file_path = temp_dir.join(file_name);
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.close(handle)?;

        // create a utf8 symlink path that points to the non-utf8 target
        let link_path = temp_dir.join("link_utf8.txt");
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.symlink(target, link, SymlinkType::File)?;

        // readlink through utf16 input should fail when output bytes are not utf8
        let link = context.path_utf16(&link_path);
        let result = context.readlink(link);
        assert!(
            result.is_err(),
            "readlink utf16 should fail for non-utf8 target bytes"
        );

        // realpath through utf16 input should fail for the same reason
        let link = context.path_utf16(&link_path);
        let result = context.realpath(link);
        assert!(
            result.is_err(),
            "realpath utf16 should fail for non-utf8 target bytes"
        );

        // cleanup
        let link = context.path_bytes(&link_path);
        context.unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
