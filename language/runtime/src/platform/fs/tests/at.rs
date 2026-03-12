use std::path::Path;

#[cfg(windows)]
use super::assert_platform_error_codes_with_privileged_policy;
use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(unix, windows))]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{
    AtFlags, FileMode, OpenFlags, OpenOptions, OpenResolveFlags, RenameFlags,
};

/// `linkat` flag bit for following a source symlink.
#[cfg(windows)]
const LINKAT_FLAG_SYMLINK_FOLLOW: u32 = 0x400;
/// `AtFlags` bit for directory removal.
#[cfg(windows)]
const AT_REMOVEDIR: u32 = 0x200;
/// One unknown `AtFlags` bit for validation tests.
#[cfg(windows)]
const AT_UNKNOWN_FLAG: u32 = 0x10;
/// `openat2` resolve bit for mount-boundary rejection.
#[cfg(windows)]
const RESOLVE_NO_XDEV: u64 = 0x1;
/// One unknown `openat2` resolve bit for validation tests.
#[cfg(windows)]
const RESOLVE_UNKNOWN: u64 = 0x40;
/// `renameat2` flag bit for exchange semantics.
#[cfg(windows)]
const RENAME_EXCHANGE: u32 = 0x2;
/// One unknown `renameat2` bit for validation tests.
#[cfg(windows)]
const RENAME_UNKNOWN: u32 = 0x8;

/// Use *at bindings for open, stat, rename, and unlink operations.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_openat_statat_renameat_unlinkat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_at");
        let file_name = Path::new("alpha.txt");
        let renamed_name = Path::new("beta.txt");
        let openat2_name = Path::new("gamma.txt");
        let renamed2_name = Path::new("delta.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // openat and write
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"openat")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // openat2 and write
        let file = context.path_bytes(openat2_name);
        let how = OpenOptions {
            flags,
            mode: FileMode(0o644),
            resolve: OpenResolveFlags(0),
        };
        let handle =
            context.destack_fs_openat2(dir_handle, file, context.open_options_value(how))?;
        let payload = context.bytes_slice_value(b"openat2")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // statat should observe the written payload length
        let file = context.path_bytes(file_name);
        let stat = context.destack_fs_statat(dir_handle, file, AtFlags(0))?;
        assert_eq!(stat.size.0, 6);

        // renameat
        let from = context.path_bytes(file_name);
        let to = context.path_bytes(renamed_name);
        context.destack_fs_renameat(dir_handle, from, dir_handle, to)?;

        // renameat2
        let from = context.path_bytes(openat2_name);
        let to = context.path_bytes(renamed2_name);
        context.destack_fs_renameat2(dir_handle, from, dir_handle, to, RenameFlags(0))?;

        // unlinkat
        let to = context.path_bytes(renamed_name);
        context.destack_fs_unlinkat(dir_handle, to, AtFlags(0))?;
        let to = context.path_bytes(renamed2_name);
        context.destack_fs_unlinkat(dir_handle, to, AtFlags(0))?;

        // cleanup
        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Read and write file contents through openat with sequential io.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_openat_sequential_read_write() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_openat_sequential");
        let file_name = Path::new("alpha.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open for write and write payload
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;
        let file = context.path_bytes(file_name);
        let write_flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let writer = context.destack_fs_openat(dir_handle, file, write_flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"openat-seq")?;
        context.destack_fs_write(writer, payload)?;
        context.destack_fs_close(writer)?;

        // reopen for read and verify sequential read works
        let file = context.path_bytes(file_name);
        let read_flags = OpenFlags(libc::O_RDONLY as u32);
        let reader = context.destack_fs_openat(dir_handle, file, read_flags, FileMode(0))?;
        let buffer = context.zeroed_bytes_slice_value(16)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let read = context.destack_fs_read(reader, buffer_call)?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, read as usize)?;
        assert_eq!(buffer, b"openat-seq");

        // cleanup
        context.destack_fs_close(reader)?;
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create a hard link through linkat and validate the linked file metadata.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_linkat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_linkat");
        let file_name = Path::new("source.txt");
        let link_name = Path::new("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;

        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"linkat")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        let existing = context.path_bytes(file_name);
        let new = context.path_bytes(link_name);
        context.destack_fs_linkat(dir_handle, existing, dir_handle, new, AtFlags(0))?;

        let link = context.path_bytes(link_name);
        let stat = context.destack_fs_statat(dir_handle, link, AtFlags(0))?;
        assert_eq!(stat.size.0, 6);

        let link = context.path_bytes(link_name);
        context.destack_fs_unlinkat(dir_handle, link, AtFlags(0))?;

        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;

        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create and resolve a symlink through symlinkat and readlinkat.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_symlinkat_readlinkat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_symlinkat");
        let file_name = Path::new("target.txt");
        let link_name = Path::new("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;

        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let target = context.path_bytes(file_name);
        let link = context.path_bytes(link_name);
        context.destack_fs_symlinkat(target, dir_handle, link, SymlinkType::File)?;

        // readlinkat should return the target path
        let link = context.path_bytes(link_name);
        let resolved = context.destack_fs_readlinkat(dir_handle, link)?;
        let name = context.path_ref_string(resolved);
        assert!(name.ends_with("target.txt"));

        let link = context.path_bytes(link_name);
        context.destack_fs_unlinkat(dir_handle, link, AtFlags(0))?;

        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;

        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Infer directory symlink targets relative to the link location on windows.
#[cfg(windows)]
#[test]
fn test_fs_symlinkat_auto_infers_directory_target_relative_to_link() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_symlinkat_auto_windows");
        let target_name = Path::new("target_dir");
        let child_name = Path::new("child.txt");
        let link_name = Path::new("link_dir");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;

        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        // create the target directory and one child file
        let target_directory = temp_dir.join(target_name);
        let directory = context.path_bytes(&target_directory);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;

        let child_path = target_directory.join(child_name);
        let child = context.path_bytes(&child_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(child, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // create the symlink with auto type inference
        let target = context.path_bytes(target_name);
        let link = context.path_bytes(link_name);
        context.destack_fs_symlinkat(target, dir_handle, link, SymlinkType::Auto)?;

        // opening one child through the symlink should behave like a directory link
        let link_child_path = temp_dir.join(link_name).join(child_name);
        let child = context.path_bytes(&link_child_path);
        let handle =
            context.destack_fs_open(child, OpenFlags(libc::O_RDONLY as u32), FileMode(0))?;
        context.destack_fs_close(handle)?;

        // cleanup
        let link = context.path_bytes(link_name);
        context.destack_fs_unlinkat(dir_handle, link, AtFlags(AT_REMOVEDIR))?;
        let child = context.path_bytes(&child_path);
        context.destack_fs_unlink(child)?;
        let directory = context.path_bytes(&target_directory);
        context.destack_fs_rmdir(directory)?;
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Exercise chmodat, chownat, and utimensat attribute updates.
#[cfg(unix)]
#[test]
fn test_fs_fchmodat_fchownat_utimensat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_attr_at");
        let file_name = Path::new("attrs.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;

        // create the file
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let file_handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        context.destack_fs_close(file_handle)?;

        // run chmodat
        let file = context.path_bytes(file_name);
        context.destack_fs_fchmodat(dir_handle, file, FileMode(0o600), AtFlags(0))?;

        // run utimensat
        let file = context.path_bytes(file_name);
        context.destack_fs_utimensat(dir_handle, file, 1_000_000_000, 2_000_000_000, AtFlags(0))?;

        // run chownat: this usually needs privileges
        let file = context.path_bytes(file_name);
        let chown_result = context.destack_fs_fchownat(dir_handle, file, 0, 0, AtFlags(0));
        context.result_ok_or_codes(
            chown_result,
            "fchownat",
            &[PlatformErrorCode::IoPermissionDenied],
        )?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unknown attribute-update flags on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_attr_at_rejects_unknown_flags_on_windows() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_attr_at_unknown_flags_windows");
        let file_name = Path::new("attrs.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(dir)?;

        // create the file
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let file_handle = context.destack_fs_openat(dir_handle, file, flags, FileMode(0o644))?;
        context.destack_fs_close(file_handle)?;

        // reject unknown bits instead of reporting generic unsupported behavior
        let file = context.path_bytes(file_name);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_fchmodat(
                dir_handle,
                file,
                FileMode(0o600),
                AtFlags(AT_UNKNOWN_FLAG),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let file = context.path_bytes(file_name);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_fchownat(dir_handle, file, 0, 0, AtFlags(AT_UNKNOWN_FLAG)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let file = context.path_bytes(file_name);
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_utimensat(
                dir_handle,
                file,
                1_000_000_000,
                2_000_000_000,
                AtFlags(AT_UNKNOWN_FLAG),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject unsupported openat2 resolve flags on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_openat2_rejects_resolve_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_openat2_flags_windows");
        let file_name = Path::new("flags.txt");
        // create parent directory and open handle
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        // reject linux openat2 resolution flags on windows
        let path = context.path_bytes(file_name);
        let options = OpenOptions {
            flags: OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32),
            mode: FileMode(0o644),
            resolve: OpenResolveFlags(RESOLVE_NO_XDEV),
        };
        let result =
            context.destack_fs_openat2(dir_handle, path, context.open_options_value(options));
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::NotSupported],
        )?;

        // cleanup
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Reject unknown openat2 resolve bits on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_openat2_rejects_unknown_resolve_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_openat2_unknown_flags_windows");
        let file_name = Path::new("flags.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        // reject unknown resolve bits instead of collapsing them into not supported
        let path = context.path_bytes(file_name);
        let options = OpenOptions {
            flags: OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32),
            mode: FileMode(0o644),
            resolve: OpenResolveFlags(RESOLVE_UNKNOWN),
        };
        let result =
            context.destack_fs_openat2(dir_handle, path, context.open_options_value(options));
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Reject renameat2 exchange semantics on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_renameat2_rejects_exchange_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_renameat2_flags_windows");
        let first_name = Path::new("first.txt");
        let second_name = Path::new("second.txt");
        // create parent directory and open handle
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        // create the first file
        let path = context.path_bytes(first_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, path, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"first")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // create the second file
        let path = context.path_bytes(second_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, path, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"second")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // reject exchange semantics on windows
        let first = context.path_bytes(first_name);
        let second = context.path_bytes(second_name);
        let result = context.destack_fs_renameat2(
            dir_handle,
            first,
            dir_handle,
            second,
            RenameFlags(RENAME_EXCHANGE),
        );
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::NotSupported],
        )?;

        // cleanup
        let first = context.path_bytes(first_name);
        context.destack_fs_unlinkat(dir_handle, first, AtFlags(0))?;
        let second = context.path_bytes(second_name);
        context.destack_fs_unlinkat(dir_handle, second, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Reject unknown renameat2 flag bits on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_renameat2_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_renameat2_unknown_flags_windows");
        let first_name = Path::new("first.txt");
        let second_name = Path::new("second.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        let path = context.path_bytes(first_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, path, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let path = context.path_bytes(second_name);
        let handle = context.destack_fs_openat(dir_handle, path, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject unknown renameat2 bits explicitly
        let first = context.path_bytes(first_name);
        let second = context.path_bytes(second_name);
        let result = context.destack_fs_renameat2(
            dir_handle,
            first,
            dir_handle,
            second,
            RenameFlags(RENAME_UNKNOWN),
        );
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let first = context.path_bytes(first_name);
        context.destack_fs_unlinkat(dir_handle, first, AtFlags(0))?;
        let second = context.path_bytes(second_name);
        context.destack_fs_unlinkat(dir_handle, second, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Reject unknown linkat flags on windows hosts.
#[cfg(windows)]
#[test]
fn test_fs_linkat_rejects_unknown_flags_on_windows() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_linkat_flags_windows");
        let file_name = Path::new("source.txt");
        let link_name = Path::new("link.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        // create the source file
        let path = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, path, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject unknown linkat flags
        let existing = context.path_bytes(file_name);
        let link = context.path_bytes(link_name);
        let result = context.destack_fs_linkat(
            dir_handle,
            existing,
            dir_handle,
            link,
            AtFlags(AT_UNKNOWN_FLAG),
        );
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}

/// Reject the known follow-symlink linkat flag on windows hosts until it is implemented.
#[cfg(windows)]
#[test]
fn test_fs_linkat_rejects_follow_symlink_flag_on_windows() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_linkat_follow_flag_windows");
        let file_name = Path::new("source.txt");
        let link_name = Path::new("link.txt");

        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.destack_fs_opendir(directory)?;

        // create the source file
        let path = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_openat(dir_handle, path, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject the known follow-symlink linkat flag explicitly
        let existing = context.path_bytes(file_name);
        let link = context.path_bytes(link_name);
        let result = context.destack_fs_linkat(
            dir_handle,
            existing,
            dir_handle,
            link,
            AtFlags(LINKAT_FLAG_SYMLINK_FOLLOW),
        );
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::NotSupported],
        )?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;
        context.destack_fs_closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(directory)?;

        Ok(())
    });
}
