//! Minimal git ignore matcher used by directory walking:
//! - comments starting with '#'
//! - blank lines
//! - negation with '!'
//! - directory-only pattern (trailing '/')
//! - '*' and '?' wildcards
//! - patterns matched against paths relative to the ignore file directory
//! - stats aware .gitattributes attributes

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::matches;

const GITIGNORE_FILE_NAME: &str = ".gitignore";
const GITATTRIBUTES_FILE_NAME: &str = ".gitattributes";
const STATS_ATTRIBUTE_NAMES: &[&str] =
    &["linguist-generated", "linguist-vendored", "export-ignore"];

/// One parsed ignore pattern.
#[derive(Debug, Clone)]
pub struct IgnorePattern {
    pub pattern: String,
    pub is_negation: bool,
    pub directory_only: bool,
}

/// Ignore rules loaded from a directory's .gitignore and .gitattributes.
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

    /// Load ignore files for a directory if not already loaded.
    ///
    /// No-op if already loaded or missing.
    pub fn load_dir(&mut self, directory: &Path) {
        let key = directory.to_path_buf();
        if self.loaded.contains_key(&key) {
            return;
        }

        // load gitignore patterns first
        let mut patterns = Vec::new();
        let ignore_file = directory.join(GITIGNORE_FILE_NAME);
        if let Ok(text) = fs::read_to_string(&ignore_file) {
            patterns.extend(parse_patterns(&text));
        }

        // load stats relevant gitattributes patterns
        let attributes_file = directory.join(GITATTRIBUTES_FILE_NAME);
        if let Ok(text) = fs::read_to_string(&attributes_file) {
            patterns.extend(parse_gitattributes_patterns(&text));
        }

        self.loaded.insert(
            key,
            IgnoreFile {
                base: directory.to_path_buf(),
                patterns,
            },
        );
    }

    /// Check if a path is ignored, considering all ancestor ignore files.
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

/// Parse .gitattributes content into ignore patterns for stats.
fn parse_gitattributes_patterns(text: &str) -> Vec<IgnorePattern> {
    let mut output = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(raw_pattern) = parts.next() else {
            continue;
        };

        // use the last stats attribute on the line
        let mut should_ignore = None;
        for attribute in parts {
            if let Some(value) = parse_stats_attribute(attribute) {
                should_ignore = Some(value);
            }
        }
        let Some(should_ignore) = should_ignore else {
            continue;
        };

        let directory_only = raw_pattern.ends_with('/');
        let pattern = if directory_only {
            &raw_pattern[..raw_pattern.len() - 1]
        } else {
            raw_pattern
        };
        if pattern.is_empty() {
            continue;
        }

        output.push(IgnorePattern {
            pattern: pattern.to_string(),
            is_negation: !should_ignore,
            directory_only,
        });
    }
    output
}

/// Parse a single stats relevant attribute value.
fn parse_stats_attribute(attribute: &str) -> Option<bool> {
    for name in STATS_ATTRIBUTE_NAMES {
        if attribute == *name {
            return Some(true);
        }
        if let Some(value) = attribute.strip_prefix('-')
            && value == *name
        {
            return Some(false);
        }
        if let Some((key, value)) = attribute.split_once('=')
            && key == *name
        {
            return parse_attribute_value(value);
        }
    }
    None
}

/// Parse a gitattributes bool-like value.
fn parse_attribute_value(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("set")
        || value.eq_ignore_ascii_case("true")
        || value.eq_ignore_ascii_case("on")
        || value.eq_ignore_ascii_case("yes")
        || value == "1"
    {
        return Some(true);
    }
    if value.eq_ignore_ascii_case("unset")
        || value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("off")
        || value.eq_ignore_ascii_case("no")
        || value == "0"
    {
        return Some(false);
    }
    None
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
        let Some(parent) = current.parent() else {
            break;
        };
        let parent_path = parent.to_path_buf();
        if parent_path == current {
            break;
        }
        current = parent_path;
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

/// Create a unix style path string with forward slashes.
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
    use std::path::Path;

    use crate::{FileSystem, TemporaryPhysicalFileSystem};

    /// Parse ignore files and match ignored paths.
    #[test]
    fn test_parse_and_match_ignore() {
        let fs = TemporaryPhysicalFileSystem::new_with_prefix("file_ignore");
        fs.create_dir_all(Path::new("project"))
            .expect("create ignore directory");
        let ignore_file = r"target/
*.log
!keep.log
# comment
";
        fs.write_bytes(
            "project/.gitignore",
            ignore_file.as_bytes(),
        )
        .expect("write ignore file");

        let mut ignore_set = IgnoreSet::new();
        ignore_set.load_dir(&fs.path_for("project"));

        let target_file = fs.path_for("project/target");
        assert!(ignore_set.is_ignored(fs.root(), &target_file, true));

        let log_file = fs.path_for("project/foo.log");
        assert!(ignore_set.is_ignored(fs.root(), &log_file, false));

        let keep_file = fs.path_for("project/keep.log");
        assert!(!ignore_set.is_ignored(fs.root(), &keep_file, false));
    }

    /// Ensure ancestor discovery stops when the filesystem root is reached.
    #[test]
    fn test_get_ancestors_between_handles_filesystem_root() {
        let root = Path::new("workspace/root");
        let ancestors = get_ancestors_between(root, Path::new("/"));
        assert_eq!(ancestors, vec![PathBuf::from("/")]);
    }

    /// Match stats ignore patterns from gitattributes.
    #[test]
    fn test_parse_and_match_gitattributes() {
        let fs = TemporaryPhysicalFileSystem::new_with_prefix("file_gitattributes");
        fs.create_dir_all(Path::new("project"))
            .expect("create attributes directory");
        let attributes_file = r"generated/parser.c linguist-generated
generated/keep.c linguist-generated
generated/keep.c -linguist-generated
*.generated.rs linguist-generated
vendor.txt linguist-vendored
archive.tar export-ignore
";
        fs.write_bytes(
            "project/.gitattributes",
            attributes_file.as_bytes(),
        )
        .expect("write attributes file");

        let mut ignore_set = IgnoreSet::new();
        ignore_set.load_dir(&fs.path_for("project"));

        let generated_file = fs.path_for("project/generated/parser.c");
        assert!(ignore_set.is_ignored(fs.root(), &generated_file, false));

        let kept_file = fs.path_for("project/generated/keep.c");
        assert!(!ignore_set.is_ignored(fs.root(), &kept_file, false));

        let generated_rust_file = fs.path_for("project/runtime/bindings.generated.rs");
        assert!(ignore_set.is_ignored(fs.root(), &generated_rust_file, false));

        let vendor_file = fs.path_for("project/vendor.txt");
        assert!(ignore_set.is_ignored(fs.root(), &vendor_file, false));

        let archive_file = fs.path_for("project/archive.tar");
        assert!(ignore_set.is_ignored(fs.root(), &archive_file, false));

        let source_file = fs.path_for("project/source.rs");
        assert!(!ignore_set.is_ignored(fs.root(), &source_file, false));
    }
}
