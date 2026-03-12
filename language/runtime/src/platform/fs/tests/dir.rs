use super::{FsDirent, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(unix, windows))]
use crate::platform::fs::SymlinkType;
use crate::platform::fs::{DirentKind, FileMode};

/// Create a directory tree and enumerate entries from an open directory handle.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdir_opendir_readdir_closedir() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_dir");
        let child_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let child = context.path_bytes(&child_dir);
        context.destack_fs_mkdir(child, FileMode(0o755))?;

        // open and read entries
        let dir = context.path_bytes(&temp_dir);
        let handle = context.destack_fs_opendir(dir)?;

        let dirents = context.destack_fs_readdir(handle)?;
        let entries = context
            .dirents_from_value(dirents)?
            .into_iter()
            .map(|entry| {
                let name = context.dirent_name(entry);
                match entry {
                    FsDirent::Native(entry) => (name, entry.kind),
                    FsDirent::Vm(entry) => (name, entry.kind),
                }
            })
            .collect::<Vec<_>>();

        // only the created child directory should appear after dot filtering
        assert!(
            entries
                .iter()
                .any(|(name, kind)| name == "child" && *kind == DirentKind::Directory)
        );
        assert!(!entries.iter().any(|(name, _)| name == "."));
        assert!(!entries.iter().any(|(name, _)| name == ".."));

        // cleanup
        context.destack_fs_closedir(handle)?;

        let child = context.path_bytes(&child_dir);
        context.destack_fs_rmdir(child)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject opening regular files through `opendir`.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_opendir_requires_directory() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_opendir_requires_directory");
        let file_path = temp_dir.join("file.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags =
            crate::platform::fs::OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // reject opening the file path as a directory
        let file = context.path_bytes(&file_path);
        super::assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_opendir(file),
            &[PlatformErrorCode::IoNotDirectory],
        )?;

        // cleanup
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create a child directory with mkdirat relative to a parent handle.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdirat_bytes() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_dirat");
        let child_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // mkdirat through directory handle
        let dir = context.path_bytes(&temp_dir);
        let handle = context.destack_fs_opendir(dir)?;

        let child_name = std::path::Path::new("child");
        let path = context.path_bytes(child_name);
        context.destack_fs_mkdirat(handle, path, FileMode(0o755))?;

        context.destack_fs_closedir(handle)?;

        // cleanup
        let child = context.path_bytes(&child_dir);
        context.destack_fs_rmdir(child)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Advance and rewind one directory-handle cursor with readdir-next semantics.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_readdir_next_and_rewinddir_cursor() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_readdir_next");
        let first_dir = temp_dir.join("alpha");

        let root = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(root, FileMode(0o755))?;
        let first = context.path_bytes(&first_dir);
        context.destack_fs_mkdir(first, FileMode(0o755))?;

        // open one directory handle and step its cursor
        let root = context.path_bytes(&temp_dir);
        let handle = context.destack_fs_opendir(root)?;
        let first_entry = context.destack_fs_readdir_next(handle)?;
        let first_entry = context.dirent_next_from_value(first_entry)?;
        let first_entry = first_entry.expect("first readdirNext should return one entry");
        let first_name = context.dirent_name(first_entry);
        assert_eq!(first_name, "alpha");

        // confirm cursor advancement to end-of-directory
        let second_entry = context.destack_fs_readdir_next(handle)?;
        let second_entry = context.dirent_next_from_value(second_entry)?;
        assert!(
            second_entry.is_none(),
            "second readdirNext should reach end"
        );

        // rewind and confirm the first entry is yielded again
        context.destack_fs_rewinddir(handle)?;
        let rewind_entry = context.destack_fs_readdir_next(handle)?;
        let rewind_entry = context.dirent_next_from_value(rewind_entry)?;
        let rewind_entry = rewind_entry.expect("rewound readdirNext should return one entry");
        let rewind_name = context.dirent_name(rewind_entry);
        assert_eq!(rewind_name, first_name);

        // cleanup
        context.destack_fs_closedir(handle)?;
        let first = context.path_bytes(&first_dir);
        context.destack_fs_rmdir(first)?;
        let root = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(root)?;

        Ok(())
    });
}

/// Report symlink entries with the symlink kind during directory enumeration.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_readdir_reports_symlink_kind() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_readdir_symlink_kind");
        let file_path = temp_dir.join("target.txt");
        let link_path = temp_dir.join("link.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create the target file
        let file = context.path_bytes(&file_path);
        let flags =
            crate::platform::fs::OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        // create the symlink, respecting Windows privilege policy
        let target = context.path_bytes(&file_path);
        let link = context.path_bytes(&link_path);
        #[cfg(unix)]
        context.destack_fs_symlink(target, link, SymlinkType::File)?;

        #[cfg(windows)]
        if let Err(error) = context.destack_fs_symlink(target, link, SymlinkType::File) {
            super::assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[PlatformErrorCode::IoPermissionDenied],
            )?;

            let file = context.path_bytes(&file_path);
            context.destack_fs_unlink(file)?;
            let dir = context.path_bytes(&temp_dir);
            context.destack_fs_rmdir(dir)?;

            return Ok(());
        }

        // enumerate entries and require the link to preserve its symlink kind
        let dir = context.path_bytes(&temp_dir);
        let handle = context.destack_fs_opendir(dir)?;
        let entries = context.destack_fs_readdir(handle)?;
        let entries = context.dirents_from_value(entries)?;
        let entries = entries
            .into_iter()
            .map(|entry| {
                let name = context.dirent_name(entry);
                let kind = match entry {
                    FsDirent::Native(entry) => entry.kind,
                    FsDirent::Vm(entry) => entry.kind,
                };
                (name, kind)
            })
            .collect::<Vec<_>>();

        assert!(
            entries
                .iter()
                .any(|(name, kind)| { name == "target.txt" && *kind == DirentKind::File })
        );
        assert!(
            entries
                .iter()
                .any(|(name, kind)| { name == "link.txt" && *kind == DirentKind::Symlink })
        );

        // cleanup
        context.destack_fs_closedir(handle)?;
        let link = context.path_bytes(&link_path);
        context.destack_fs_unlink(link)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Continue full-directory reads from the current shared cursor.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_readdir_respects_existing_cursor_position() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_readdir_cursor_resume");
        let first_dir = temp_dir.join("alpha");
        let second_dir = temp_dir.join("beta");

        let root = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(root, FileMode(0o755))?;
        let first = context.path_bytes(&first_dir);
        context.destack_fs_mkdir(first, FileMode(0o755))?;
        let second = context.path_bytes(&second_dir);
        context.destack_fs_mkdir(second, FileMode(0o755))?;

        // advance the shared cursor by one entry
        let root = context.path_bytes(&temp_dir);
        let handle = context.destack_fs_opendir(root)?;
        let first_entry = context.destack_fs_readdir_next(handle)?;
        let first_entry = context.dirent_next_from_value(first_entry)?;
        let first_entry = first_entry.expect("first readdirNext should return one entry");
        let first_name = context.dirent_name(first_entry);

        // require readdir to resume from that cursor instead of restarting
        let remaining = context.destack_fs_readdir(handle)?;
        let remaining = context
            .dirents_from_value(remaining)?
            .into_iter()
            .map(|entry| context.dirent_name(entry))
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 1);
        assert_ne!(remaining[0], first_name);
        let expected_remaining = if first_name == "alpha" {
            "beta"
        } else {
            "alpha"
        };
        assert_eq!(remaining[0], expected_remaining);

        // cleanup
        context.destack_fs_closedir(handle)?;
        let second = context.path_bytes(&second_dir);
        context.destack_fs_rmdir(second)?;
        let first = context.path_bytes(&first_dir);
        context.destack_fs_rmdir(first)?;
        let root = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(root)?;

        Ok(())
    });
}

/// Create a unique temporary directory from a mkdtemp template.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdtemp() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mkdtemp");
        let custom_parent = temp_dir.join("custom_parent");
        let template_path = custom_parent.join("destack_XXXXXX");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let parent = context.path_bytes(&custom_parent);
        context.destack_fs_mkdir(parent, FileMode(0o755))?;

        // create a temporary directory
        let template = context.path_bytes(&template_path);
        let created = context.destack_fs_mkdtemp(template)?;
        let created_path = std::path::PathBuf::from(context.path_ref_string(created));
        assert_eq!(created_path.parent(), Some(custom_parent.as_path()));
        let created_name = created_path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("mkdtemp should return a utf8 file name in this test");
        assert!(created_name.starts_with("destack_"));
        assert_eq!(created_name.len(), "destack_".len() + 6);

        // cleanup
        let created = context.path_bytes(&created_path);
        context.destack_fs_rmdir(created)?;
        let parent = context.path_bytes(&custom_parent);
        context.destack_fs_rmdir(parent)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject mkdtemp templates that do not end with the required suffix.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdtemp_rejects_non_trailing_suffix() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mkdtemp_invalid_suffix");
        let template_path = temp_dir.join("destack_XXXXXX_suffix");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // reject templates with a non trailing marker
        let template = context.path_bytes(&template_path);
        super::assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_mkdtemp(template),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
