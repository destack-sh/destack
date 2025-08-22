//! Minimal .gitignore-like matcher used by directory walking.
//! Supports:
//! - comments starting with '#'
//! - blank lines
//! - negation with '!'
//! - directory-only pattern (trailing '/')
//! - '*' and '?' wildcards
//! - patterns matched against paths relative to the ignore file directory

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::glob::matches;

/// One parsed ignore pattern.
#[derive(Debug, Clone)]
pub struct IgnorePattern {
    pub pattern: String,
    pub is_negation: bool,
    pub directory_only: bool,
}

/// Ignore rules loaded from a directory's .gitignore.
#[derive(Debug, Clone)]
pub struct IgnoreFile {
    pub base: PathBuf,
    pub patterns: Vec<IgnorePattern>,
}

/// A set of loaded ignore files, indexed by base directory.
#[derive(Debug, Default)]
pub struct IgnoreSet {
    loaded: HashMap<PathBuf, IgnoreFile>,
}

impl IgnoreSet {
    /// Create a new, empty set.
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }

    /// Load ignore file for a directory if not already loaded.
    ///
    /// No-op if already loaded or missing.
    pub fn load_dir(&mut self, directory: &Path) {
        let key = directory.to_path_buf();
        if self.loaded.contains_key(&key) {
            return;
        }
        let ignore_file = directory.join(".gitignore");
        let Ok(text) = fs::read_to_string(&ignore_file) else {
            // mark as checked to avoid repeated io
            self.loaded.insert(
                key,
                IgnoreFile {
                    base: directory.to_path_buf(),
                    patterns: Vec::new(),
                },
            );
            return;
        };
        let patterns = parse_patterns(&text);
        self.loaded.insert(
            key,
            IgnoreFile {
                base: directory.to_path_buf(),
                patterns,
            },
        );
    }

    /// Check if a path is ignored, considering all ancestor .gitignore files.
    pub fn is_ignored(&self, root: &Path, path: &Path, is_directory: bool) -> bool {
        let mut ignored = false;
        let ancestors = get_ancestors_between(root, path.parent().unwrap_or(root));
        for base in ancestors {
            if let Some(ignore_file) = self.loaded.get(&base) {
                let relative_path = strip_prefix(path, &ignore_file.base).unwrap_or(path);
                let relative_path_string = unix_path(relative_path);
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                for pattern in &ignore_file.patterns {
                    if pattern.directory_only && !is_directory {
                        continue;
                    }
                    if pattern_matches(&pattern.pattern, &relative_path_string, file_name) {
                        ignored = !pattern.is_negation;
                    }
                }
            }
        }
        ignored
    }
}

/// Parse .gitignore file content into patterns.
fn parse_patterns(text: &str) -> Vec<IgnorePattern> {
    let mut output = Vec::new();
    for raw_line in text.lines() {
        let mut line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut is_negation = false;
        if let Some(rest) = line.strip_prefix('!') {
            is_negation = true;
            line = rest.trim();
            if line.is_empty() {
                continue;
            }
        }
        let directory_only = line.ends_with('/');
        let pattern = if directory_only {
            &line[..line.len() - 1]
        } else {
            line
        };
        output.push(IgnorePattern {
            pattern: pattern.to_string(),
            is_negation,
            directory_only,
        });
    }
    output
}

/// Check if pattern matches either the relative path or file name when pattern has no slash.
fn pattern_matches(pattern: &str, relative_path: &str, file_name: &str) -> bool {
    if pattern.contains('/') {
        matches(pattern.as_bytes(), 0, relative_path.as_bytes(), 0)
    } else {
        matches(pattern.as_bytes(), 0, file_name.as_bytes(), 0)
    }
}

/// Get vector of ancestors between root and end (inclusive of root and end).
fn get_ancestors_between(root: &Path, end: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let root_cleaned = clean_path(root);
    let mut current = clean_path(end);
    let mut stack = Vec::new();
    loop {
        stack.push(current.clone());
        if current == root_cleaned {
            break;
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    while let Some(path) = stack.pop() {
        result.push(path);
    }
    result
}

/// Normalize a path by removing '.' and '..' components.
fn clean_path(path: &Path) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                output.pop();
            }
            _ => output.push(component.as_os_str()),
        }
    }
    output
}

/// Create a unix-style path string (forward slashes).
fn unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Strip prefix from path, returning None when not possible.
fn strip_prefix<'a>(path: &'a Path, base: &Path) -> Option<&'a Path> {
    path.strip_prefix(base).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_and_match_ignore() {
        let temp_directory = tempdir();
        let subdirectory = temp_directory.join("project");
        let _ = fs::create_dir_all(&subdirectory);
        let ignore_file = subdirectory.join(".gitignore");
        let content = b"target/\n*.log\n!keep.log\n# comment\n";
        let mut file = fs::File::create(&ignore_file).unwrap();
        file.write_all(content).unwrap();

        let mut ignore_set = IgnoreSet::new();
        ignore_set.load_dir(&subdirectory);

        let target_file = subdirectory.join("target");
        assert!(ignore_set.is_ignored(&temp_directory, &target_file, true));

        let log_file = subdirectory.join("foo.log");
        assert!(ignore_set.is_ignored(&temp_directory, &log_file, false));

        let keep_file = subdirectory.join("keep.log");
        assert!(!ignore_set.is_ignored(&temp_directory, &keep_file, false));
    }

    fn tempdir() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "destack_fs_ignore_{}",
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        ));
        let _ = fs::create_dir_all(&path);
        path
    }
}
