use super::{FsDirent, temp_dir, with_harness_context};
use crate::platform::fs::{DirentKind, FileMode};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdir_opendir_readdir_closedir() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_dir");
        let child_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;
        let child = context.path_bytes(&child_dir);
        context.mkdir(child, FileMode(0o755))?;

        // open and read entries
        let dir = context.path_bytes(&temp_dir);
        let handle = context.opendir(dir)?;

        let entries = context
            .readdir(handle)?
            .into_iter()
            .map(|entry| {
                let name = context.dirent_name(entry);
                match entry {
                    FsDirent::Native(entry) => (name, entry.kind),
                    FsDirent::Vm(entry) => (name, entry.kind),
                }
            })
            .collect::<Vec<_>>();

        assert!(
            entries
                .iter()
                .any(|(name, kind)| name == "child" && *kind == DirentKind::Directory)
        );
        assert!(!entries.iter().any(|(name, _)| name == "."));
        assert!(!entries.iter().any(|(name, _)| name == ".."));

        // cleanup
        context.closedir(handle)?;

        let child = context.path_bytes(&child_dir);
        context.rmdir(child)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdirat_bytes() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_dirat");
        let child_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // mkdirat through directory handle
        let dir = context.path_bytes(&temp_dir);
        let handle = context.opendir(dir)?;

        let child_name = std::path::Path::new("child");
        let path = context.path_bytes(child_name);
        context.mkdirat(handle, path, FileMode(0o755))?;

        context.closedir(handle)?;

        // cleanup
        let child = context.path_bytes(&child_dir);
        context.rmdir(child)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mkdtemp() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mkdtemp");
        let template_path = temp_dir.join("destack_XXXXXX");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        // create a temporary directory
        let template = context.path_bytes(&template_path);
        let created = context.mkdtemp(template)?;
        let name = context.path_ref_string(created.clone());
        assert!(name.contains("destack_"));

        // cleanup
        context.rmdir(created)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
