use super::{temp_dir, with_harness_context};
#[cfg(any(unix, windows))]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{CopyFlags, FileMode, OpenFlags};
#[cfg(any(unix, windows))]
use crate::tests::platform::assert_platform_error_codes_with_privileged_policy;

/// Copyfile flag to reject replacing an existing destination.
const COPYFILE_FAIL_IF_EXISTS: u32 = 0x1;
/// One unsupported copyfile flag bit for validation tests.
#[cfg(windows)]
const COPYFILE_UNKNOWN_FLAG: u32 = 0x2;

/// Rename, hard link, and copy files while preserving payload bytes.
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
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write initial file
        let path = context.path_bytes(&file_a);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(path, flags, FileMode(0o644))?;
        let payload = b"destack".to_vec();
        let payload_value = context.bytes_slice_value(&payload)?;
        context.destack_fs_write(handle, payload_value)?;
        context.destack_fs_close(handle)?;

        // rename, link, and copy
        let from = context.path_bytes(&file_a);
        let to = context.path_bytes(&file_b);
        context.destack_fs_rename(from, to)?;

        let from = context.path_bytes(&file_b);
        let to = context.path_bytes(&file_c);
        context.destack_fs_link(from, to)?;

        let from = context.path_bytes(&file_b);
        let to = context.path_bytes(&file_d);
        context.destack_fs_copyfile(from, to, CopyFlags(0))?;

        // linked file should contain the original payload
        let path = context.path_bytes(&file_c);
        let flags = OpenFlags(libc::O_RDONLY as u32);
        let handle = context.destack_fs_open(path, flags, FileMode(0o644))?;
        let buffer = context.zeroed_bytes_slice_value(16)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_read(handle, buffer_call)?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"destack");
        context.destack_fs_close(handle)?;

        // copied file should contain the original payload
        let path = context.path_bytes(&file_d);
        let handle = context.destack_fs_open(path, flags, FileMode(0o644))?;
        let buffer = context.zeroed_bytes_slice_value(16)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_read(handle, buffer_call)?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"destack");
        context.destack_fs_close(handle)?;

        // cleanup
        let to = context.path_bytes(&file_b);
        context.destack_fs_unlink(to)?;
        let to = context.path_bytes(&file_c);
        context.destack_fs_unlink(to)?;
        let to = context.path_bytes(&file_d);
        context.destack_fs_unlink(to)?;

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject replacing an existing destination when copyfile uses the no replace flag.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_copyfile_respects_fail_if_exists_flag() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_copyfile_noreplace");
        let source_path = temp_dir.join("source.txt");
        let target_path = temp_dir.join("target.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;

        // seed the source and destination payloads
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);

        let source = context.path_bytes(&source_path);
        let handle = context.destack_fs_open(source, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"source")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        let target = context.path_bytes(&target_path);
        let handle = context.destack_fs_open(target, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"target")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // reject replacing the existing destination
        let source = context.path_bytes(&source_path);
        let target = context.path_bytes(&target_path);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_copyfile(source, target, CopyFlags(COPYFILE_FAIL_IF_EXISTS)),
            &[PlatformErrorCode::IoAlreadyExists],
        )?;

        // verify the destination payload was preserved
        let target = context.path_bytes(&target_path);
        let handle =
            context.destack_fs_open(target, OpenFlags(libc::O_RDONLY as u32), FileMode(0))?;
        let buffer = context.zeroed_bytes_slice_value(16)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_read(handle, buffer_call)?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"target");
        context.destack_fs_close(handle)?;

        // cleanup
        let target = context.path_bytes(&target_path);
        context.destack_fs_unlink(target)?;
        let source = context.path_bytes(&source_path);
        context.destack_fs_unlink(source)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Preserve source permission bits when copyfile creates the destination on unix.
#[cfg(unix)]
#[test]
fn test_fs_copyfile_preserves_source_mode_bits() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_copyfile_mode");
        let source_path = temp_dir.join("source.txt");
        let target_path = temp_dir.join("target.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;

        // create the source and force a mode that differs from the common process umask
        let source = context.path_bytes(&source_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(source, flags, FileMode(0o600))?;
        context.destack_fs_close(handle)?;

        let source = context.path_bytes(&source_path);
        context.destack_fs_chmod(source, FileMode(0o777))?;

        // copy the source into a new destination
        let source = context.path_bytes(&source_path);
        let target = context.path_bytes(&target_path);
        context.destack_fs_copyfile(source, target, CopyFlags(0))?;

        // require the destination mode bits to match the source permission bits exactly
        let source = context.path_bytes(&source_path);
        let source_stat = context.destack_fs_stat(source)?;

        let target = context.path_bytes(&target_path);
        let target_stat = context.destack_fs_stat(target)?;
        assert_eq!(source_stat.mode.0 & 0o777, 0o777);
        assert_eq!(target_stat.mode.0 & 0o777, source_stat.mode.0 & 0o777);

        // cleanup
        let target = context.path_bytes(&target_path);
        context.destack_fs_unlink(target)?;
        let source = context.path_bytes(&source_path);
        context.destack_fs_unlink(source)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Create a symlink and read back its target path bytes.
#[cfg(unix)]
#[test]
fn test_fs_symlink_readlink() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_symlink");
        let file_path = temp_dir.join("file.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create target file
        let path = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(path, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // symlink and readlink
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.destack_fs_symlink(target, link, SymlinkType::File)?;

        let link = context.path_bytes(&link_path);
        let path = context.destack_fs_readlink(link)?;
        let bytes = context.path_ref_bytes(path);
        assert!(bytes.ends_with(b"file.txt"));

        // cleanup
        let link = context.path_bytes(&link_path);
        context.destack_fs_unlink(link)?;

        let path = context.path_bytes(&file_path);
        context.destack_fs_unlink(path)?;

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Resolve a canonical path for an existing file.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_realpath() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_realpath");
        let file_path = temp_dir.join("real.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create file
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // resolve real path
        let file = context.path_bytes(&file_path);
        let resolved = context.destack_fs_realpath(file)?;
        let name = context.path_ref_string(resolved);
        assert!(name.ends_with("real.txt"));

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Preserve non-utf8 bytes when resolving realpath and readlink on unix.
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
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create a file with a non-utf8 byte in the name
        let file_name = OsString::from_vec(b"target_\xff.bin".to_vec());
        let file_path = temp_dir.join(file_name);
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // resolve realpath and ensure the non-utf8 byte survives
        let file = context.path_bytes(&file_path);
        let resolved = context.destack_fs_realpath(file)?;
        let resolved = context.path_ref_bytes(resolved);
        assert!(resolved.contains(&0xff));

        // create a symlink and verify readlink bytes preserve the target bytes
        let link_name = OsString::from_vec(b"link_\xfe.bin".to_vec());
        let link_path = temp_dir.join(link_name);
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.destack_fs_symlink(target, link, SymlinkType::File)?;

        let link = context.path_bytes(&link_path);
        let linked_target = context.destack_fs_readlink(link)?;
        let linked_target = context.path_ref_bytes(linked_target);
        let expected = file_path.as_os_str().as_bytes();
        assert_eq!(linked_target.as_slice(), expected);

        // cleanup
        let link = context.path_bytes(&link_path);
        context.destack_fs_unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Accept utf16 input paths on unix by routing through utf8 byte paths.
#[cfg(unix)]
#[test]
fn test_fs_utf16_input_path_on_unix_uses_utf8_bytes() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_utf16_unix");
        let file_path = temp_dir.join("utf8_name.txt");

        // create directory
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create file through utf16 path encoding
        let file = context.path_utf16(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"payload")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // stat and resolve through utf16 path encoding
        let file = context.path_utf16(&file_path);
        let stat = context.destack_fs_stat(file)?;
        assert_eq!(stat.size.0, 7);

        let file = context.path_utf16(&file_path);
        let resolved = context.destack_fs_realpath(file)?;
        let resolved = context.path_ref_string(resolved);
        assert!(resolved.ends_with("utf8_name.txt"));

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject utf16 output path decoding on unix for non-utf8 targets.
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
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create a file with non-utf8 bytes in the name
        let file_name = OsString::from_vec(b"target_\xff.bin".to_vec());
        let file_path = temp_dir.join(file_name);
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // create a utf8 symlink path that points to the non-utf8 target
        let link_path = temp_dir.join("link_utf8.txt");
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        context.destack_fs_symlink(target, link, SymlinkType::File)?;

        // utf16 readlink cannot encode non-utf8 host output
        let link = context.path_utf16(&link_path);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_readlink(link),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // utf16 realpath cannot encode non-utf8 host output
        let link = context.path_utf16(&link_path);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_realpath(link),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let link = context.path_bytes(&link_path);
        context.destack_fs_unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unsupported copyfile flags on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_copyfile_rejects_unknown_flags_on_windows() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_copyfile_flags_windows");
        let source_path = temp_dir.join("source.txt");
        let target_path = temp_dir.join("target.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;

        // create the source file
        let source = context.path_bytes(&source_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(source, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"copy")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // reject unsupported copy flags
        let source = context.path_bytes(&source_path);
        let target = context.path_bytes(&target_path);
        let result = context.destack_fs_copyfile(source, target, CopyFlags(COPYFILE_UNKNOWN_FLAG));
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let source = context.path_bytes(&source_path);
        context.destack_fs_unlink(source)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}
