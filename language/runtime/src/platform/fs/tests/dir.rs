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
        let name = context.path_ref_string(created.clone());
        assert!(name.contains("destack_"));

        // cleanup
        context.destack_fs_rmdir(created)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
