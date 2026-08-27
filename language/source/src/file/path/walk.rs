use std::io;
use std::path::{Path, PathBuf};

use crate::FileSystem;

use super::IgnoreSet;

/// Options for directory walking.
#[derive(Debug, Clone, Default)]
pub struct WalkOptions {
    /// Root directory to walk.
    pub root: PathBuf,
    /// Directory names to skip (exact match of final component).
    pub ignore: Option<Vec<String>>,
    /// Optional glob-like path patterns to include (matched on full path string).
    pub glob: Option<Vec<String>>,
}

/// Visit files below one root in depth-first order.
pub fn walk<F>(
    file_system: &dyn FileSystem,
    options: &WalkOptions,
    mut visitor: F,
) -> io::Result<()>
where
    F: FnMut(&Path),
{
    // retain pending directories without recursive calls
    let mut directory_stack: Vec<PathBuf> = vec![options.root.clone()];
    let mut ignore_set = IgnoreSet::new();

    while let Some(directory) = directory_stack.pop() {
        // load ignore rules declared by this directory
        ignore_set.load(file_system, &directory)?;

        // inspect every directory entry through the selected file system
        let paths = file_system.read_dir(&directory).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to read {}: {error}", directory.display()),
            )
        })?;
        for path in paths {
            let metadata = file_system.symlink_metadata(&path).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("failed to inspect {}: {error}", path.display()),
                )
            })?;

            // skip ignored entries
            if ignore_set.is_ignored(&options.root, &path, metadata.is_directory) {
                continue;
            }

            // schedule included directories
            if metadata.is_directory {
                if let Some(name) = path.file_name().and_then(|s| s.to_str())
                    && let Some(ignore) = &options.ignore
                    && ignore.iter().any(|directory| directory == name)
                {
                    continue;
                }
                directory_stack.push(path);
                continue;
            }
            if !metadata.is_file {
                continue;
            }

            // skip files outside the selected patterns
            if let Some(glob) = &options.glob {
                let text = path.to_string_lossy();
                if !glob
                    .iter()
                    .any(|pattern| super::matches(pattern.as_bytes(), text.as_bytes()))
                {
                    continue;
                }
            }

            // visit the selected file
            visitor(&path);
        }
    }

    Ok(())
}
