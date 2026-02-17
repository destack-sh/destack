use std::path::Path;

use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{
    AtFlags, FileMode, OpenFlags, OpenOptions, OpenResolveFlags, RenameFlags,
};

/// Use *at bindings for open, stat, rename, and unlink operations.
#[cfg(unix)]
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
        #[cfg(unix)]
        context.destack_fs_linkat(dir_handle, existing, dir_handle, new, AtFlags(0))?;
        #[cfg(windows)]
        let allowed = [PlatformErrorCode::NotSupported];
        #[cfg(windows)]
        let link_result =
            context.destack_fs_linkat(dir_handle, existing, dir_handle, new, AtFlags(0));
        #[cfg(windows)]
        let _ = context.result_ok_or_codes(link_result, "linkat", &allowed)?;

        #[cfg(unix)]
        {
            let link = context.path_bytes(link_name);
            let stat = context.destack_fs_statat(dir_handle, link, AtFlags(0))?;
            assert_eq!(stat.size.0, 6);

            let link = context.path_bytes(link_name);
            context.destack_fs_unlinkat(dir_handle, link, AtFlags(0))?;
        }
        #[cfg(windows)]
        {
            let allowed = [PlatformErrorCode::IoNotFound];
            let link = context.path_bytes(link_name);
            let unlink_result = context.destack_fs_unlinkat(dir_handle, link, AtFlags(0));
            let _ = context.result_ok_or_codes(unlink_result, "unlinkat(link)", &allowed)?;
        }

        let file = context.path_bytes(file_name);
        context.destack_fs_unlinkat(dir_handle, file, AtFlags(0))?;

        context.destack_fs_closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create and resolve a symlink through symlinkat and readlinkat.
#[cfg(unix)]
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
            &[PlatformErrorCode::IoPermissionDenied, PlatformErrorCode::Io],
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
            resolve: OpenResolveFlags(0x1),
        };
        let result =
            context.destack_fs_openat2(dir_handle, path, context.open_options_value(options));
        super::assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;

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
        let result =
            context.destack_fs_renameat2(dir_handle, first, dir_handle, second, RenameFlags(0x2));
        super::assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;

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
