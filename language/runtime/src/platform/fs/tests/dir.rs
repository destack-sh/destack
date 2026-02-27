use super::{FsDirent, temp_dir, with_harness_context};
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

/// Create a unique temporary directory from a mkdtemp template.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdtemp() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mkdtemp");
        let template_path = temp_dir.join("destack_XXXXXX");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create a temporary directory
        let template = context.path_bytes(&template_path);
        let created = context.destack_fs_mkdtemp(template)?;
        let name = context.path_ref_string(created);
        assert!(name.contains("destack_"));

        // cleanup
        context.destack_fs_rmdir(created)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
