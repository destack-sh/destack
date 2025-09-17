//! Generic directory walker with filtering hooks and ignore support.

use std::path::{Path, PathBuf};

use crate::ignore::IgnoreSet;

/// Options for directory walking.
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Root directory to walk.
    pub root: PathBuf,
    /// Directory names to skip (exact match of final component).
    pub ignore: Option<Vec<String>>,
    /// Optional glob-like path patterns to include (matched on full path string).
    pub glob: Option<Vec<String>>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            ignore: None,
            glob: None,
        }
    }
}

/// Visit files under a root use a stack-based DFS, applying ignore rules and filters.
///
/// The visitor closure receives `&Path` of a file.
/// Errors are ignored.
pub fn walk<F>(options: &WalkOptions, mut visitor: F)
where
    F: FnMut(&Path),
{
    // stack-based DFS to avoid recursion
    let mut dir_stack: Vec<PathBuf> = vec![options.root.clone()];
    let mut ignore_set = IgnoreSet::new();

    while let Some(dir) = dir_stack.pop() {
        // consider ignore set
        ignore_set.load_dir(&dir);

        // walk entries
        let read_dir = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let is_dir = file_type.is_dir();

            // skip if ignored
            if ignore_set.is_ignored(&options.root, &path, is_dir) {
                continue;
            }

            // if directory, add to stack
            if is_dir {
                if let Some(name) = path.file_name().and_then(|s| s.to_str())
                    && options
                        .ignore
                        .as_ref()
                        .unwrap_or(&vec![])
                        .iter()
                        .any(|d| d == name)
                {
                    continue;
                }
                dir_stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            // skip if not matched by glob
            if let Some(glob) = &options.glob {
                let text = path.to_string_lossy();
                if !glob
                    .iter()
                    .any(|p| crate::glob::matches(p.as_bytes(), 0, text.as_bytes(), 0))
                {
                    continue;
                }
            }

            // visit file
            visitor(&path);
        }
    }
}
