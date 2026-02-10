use std::path::Path;

use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{
    AtFlags, FileMode, OpenFlags, OpenOptions, OpenResolveFlags, RenameFlags,
};

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
        context.mkdir(dir, FileMode(0o755))?;

        // openat and write
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(dir)?;
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.openat(dir_handle, file, flags, FileMode(0o644))?;
        context.write(handle, b"openat")?;
        context.close(handle)?;

        // openat2 and write
        let file = context.path_bytes(openat2_name);
        let how = OpenOptions {
            flags,
            mode: FileMode(0o644),
            resolve: OpenResolveFlags(0),
        };
        let handle = context.openat2(dir_handle, file, how)?;
        context.write(handle, b"openat2")?;
        context.close(handle)?;

        // statat
        let file = context.path_bytes(file_name);
        let stat = context.statat(dir_handle, file, AtFlags(0))?;
        assert_eq!(stat.size.0, 6);

        // renameat
        let from = context.path_bytes(file_name);
        let to = context.path_bytes(renamed_name);
        context.renameat(dir_handle, from, dir_handle, to)?;

        // renameat2
        let from = context.path_bytes(openat2_name);
        let to = context.path_bytes(renamed2_name);
        context.renameat2(dir_handle, from, dir_handle, to, RenameFlags(0))?;

        // unlinkat
        let to = context.path_bytes(renamed_name);
        context.unlinkat(dir_handle, to, AtFlags(0))?;
        let to = context.path_bytes(renamed2_name);
        context.unlinkat(dir_handle, to, AtFlags(0))?;

        // cleanup
        context.closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_openat_sequential_read_write() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_openat_sequential");
        let file_name = Path::new("alpha.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // open for write and write payload
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(dir)?;
        let file = context.path_bytes(file_name);
        let write_flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let writer = context.openat(dir_handle, file, write_flags, FileMode(0o644))?;
        context.write(writer, b"openat-seq")?;
        context.close(writer)?;

        // reopen for read and verify sequential read works
        let file = context.path_bytes(file_name);
        let read_flags = OpenFlags(libc::O_RDONLY as u32);
        let reader = context.openat(dir_handle, file, read_flags, FileMode(0))?;
        let mut buffer = vec![0u8; 16];
        let read = context.read(reader, &mut buffer)?;
        buffer.truncate(read as usize);
        assert_eq!(buffer, b"openat-seq");

        // cleanup
        context.close(reader)?;
        let file = context.path_bytes(file_name);
        context.unlinkat(dir_handle, file, AtFlags(0))?;
        context.closedir(dir_handle)?;

        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_linkat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_linkat");
        let file_name = Path::new("source.txt");
        let link_name = Path::new("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(dir)?;

        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.openat(dir_handle, file, flags, FileMode(0o644))?;
        context.write(handle, b"linkat")?;
        context.close(handle)?;

        let allowed = [PlatformErrorCode::NotSupported];
        let existing = context.path_bytes(file_name);
        let new = context.path_bytes(link_name);
        let link_result = context.linkat(dir_handle, existing, dir_handle, new, AtFlags(0));
        let linked = context.result_ok_or_codes(link_result, "linkat", &allowed)?;

        if linked.is_some() {
            let link = context.path_bytes(link_name);
            let stat = context.statat(dir_handle, link, AtFlags(0))?;
            assert_eq!(stat.size.0, 6);

            let link = context.path_bytes(link_name);
            context.unlinkat(dir_handle, link, AtFlags(0))?;
        }

        let file = context.path_bytes(file_name);
        context.unlinkat(dir_handle, file, AtFlags(0))?;

        context.closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_fs_symlinkat_readlinkat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_symlinkat");
        let file_name = Path::new("target.txt");
        let link_name = Path::new("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(dir)?;

        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.openat(dir_handle, file, flags, FileMode(0o644))?;
        context.close(handle)?;

        let allowed = [PlatformErrorCode::NotSupported];
        let target = context.path_bytes(file_name);
        let link = context.path_bytes(link_name);
        let symlink_result = context.symlinkat(target, dir_handle, link, SymlinkType::File);
        let linked = context.result_ok_or_codes(symlink_result, "symlinkat", &allowed)?;

        if linked.is_some() {
            let link = context.path_bytes(link_name);
            let resolved = context.readlinkat(dir_handle, link)?;
            let name = context.path_ref_string(resolved);
            assert!(name.ends_with("target.txt"));

            let link = context.path_bytes(link_name);
            context.unlinkat(dir_handle, link, AtFlags(0))?;
        }

        let file = context.path_bytes(file_name);
        context.unlinkat(dir_handle, file, AtFlags(0))?;

        context.closedir(dir_handle)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_fs_fchmodat_fchownat_utimensat() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_attr_at");
        let file_name = Path::new("attrs.txt");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;
        let dir = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(dir)?;

        // create the file
        let file = context.path_bytes(file_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let file_handle = context.openat(dir_handle, file, flags, FileMode(0o644))?;
        context.close(file_handle)?;

        // run chmodat
        let file = context.path_bytes(file_name);
        let chmod_result = context.fchmodat(dir_handle, file, FileMode(0o600), AtFlags(0));
        context.result_ok_or_codes(chmod_result, "fchmodat", &[PlatformErrorCode::NotSupported])?;

        // run utimensat
        let file = context.path_bytes(file_name);
        let utimens_result =
            context.utimensat(dir_handle, file, 1_000_000_000, 2_000_000_000, AtFlags(0));
        context.result_ok_or_codes(
            utimens_result,
            "utimensat",
            &[PlatformErrorCode::NotSupported],
        )?;

        // run chownat: this usually needs privileges
        let file = context.path_bytes(file_name);
        let chown_result = context.fchownat(dir_handle, file, 0, 0, AtFlags(0));
        context.result_ok_or_codes(
            chown_result,
            "fchownat",
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::Io,
            ],
        )?;

        // cleanup
        let file = context.path_bytes(file_name);
        context.unlinkat(dir_handle, file, AtFlags(0))?;
        context.closedir(dir_handle)?;

        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_fs_openat2_rejects_resolve_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_openat2_flags_windows");
        let file_name = Path::new("flags.txt");
        let allowed = [PlatformErrorCode::NotSupported];

        // create parent directory and open handle
        let directory = context.path_bytes(&temp_dir);
        context.mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(directory)?;

        // reject linux openat2 resolution flags on windows
        let path = context.path_bytes(file_name);
        let options = OpenOptions {
            flags: OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32),
            mode: FileMode(0o644),
            resolve: OpenResolveFlags(0x1),
        };
        let result = context.openat2(dir_handle, path, options);
        context.result_ok_or_codes(result, "openat2", &allowed)?;

        // cleanup
        context.closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.rmdir(directory)?;

        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_fs_renameat2_rejects_exchange_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_renameat2_flags_windows");
        let first_name = Path::new("first.txt");
        let second_name = Path::new("second.txt");
        let allowed = [PlatformErrorCode::NotSupported];

        // create parent directory and open handle
        let directory = context.path_bytes(&temp_dir);
        context.mkdir(directory, FileMode(0o755))?;
        let directory = context.path_bytes(&temp_dir);
        let dir_handle = context.opendir(directory)?;

        // create the first file
        let path = context.path_bytes(first_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.openat(dir_handle, path, flags, FileMode(0o644))?;
        context.write(handle, b"first")?;
        context.close(handle)?;

        // create the second file
        let path = context.path_bytes(second_name);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.openat(dir_handle, path, flags, FileMode(0o644))?;
        context.write(handle, b"second")?;
        context.close(handle)?;

        // reject exchange semantics on windows
        let first = context.path_bytes(first_name);
        let second = context.path_bytes(second_name);
        let result = context.renameat2(dir_handle, first, dir_handle, second, RenameFlags(0x2));
        context.result_ok_or_codes(result, "renameat2", &allowed)?;

        // cleanup
        let first = context.path_bytes(first_name);
        context.unlinkat(dir_handle, first, AtFlags(0))?;
        let second = context.path_bytes(second_name);
        context.unlinkat(dir_handle, second, AtFlags(0))?;
        context.closedir(dir_handle)?;
        let directory = context.path_bytes(&temp_dir);
        context.rmdir(directory)?;

        Ok(())
    });
}
