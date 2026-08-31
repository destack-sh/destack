use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::{FileSystem, PathExt};

use super::matches;

const GITIGNORE_FILE_NAME: &str = ".gitignore";
const GITATTRIBUTES_FILE_NAME: &str = ".gitattributes";
const GIT_DIRECTORY_NAME: &str = ".git";
const GIT_COMMON_DIRECTORY_FILE_NAME: &str = "commondir";
const GIT_EXCLUDE_PATH: &str = "info/exclude";
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

    /// Load root ignore files and repository-local Git exclusions.
    pub fn load_root(&mut self, file_system: &dyn FileSystem, root: &Path) -> io::Result<()> {
        // find the enclosing Git repository
        let mut repository = None;
        for directory in root.ancestors() {
            let git_path = directory.join(GIT_DIRECTORY_NAME);
            match file_system.symlink_metadata(&git_path) {
                Ok(metadata) => {
                    repository = Some((directory, git_path, metadata));
                    break;
                }
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        let Some((repository_root, git_path, git_metadata)) = repository else {
            return self.load(file_system, root);
        };

        // load inherited ignore files through the walk root
        let relative_root = root.strip_prefix(repository_root).map_err(|_| {
            io::Error::other(format!(
                "source root is outside Git repository: {}",
                root.display()
            ))
        })?;
        let mut directory = repository_root.to_path_buf();
        self.load(file_system, &directory)?;
        for component in relative_root.components() {
            directory.push(component);
            self.load(file_system, &directory)?;
        }

        // resolve the Git directory for regular and linked worktrees
        let git_directory = if git_metadata.is_directory {
            git_path
        } else if git_metadata.is_file {
            let declaration = file_system.read_to_string(&git_path)?;
            let Some(path) = declaration.trim().strip_prefix("gitdir:") else {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid Git directory declaration: {}", git_path.display()),
                ));
            };
            let path = path.trim();
            if path.is_empty() {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("empty Git directory declaration: {}", git_path.display()),
                ));
            }
            let path = Path::new(path);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                repository_root.normalize_with(path)
            }
        } else {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("invalid Git directory entry: {}", git_path.display()),
            ));
        };

        // resolve the shared Git directory for linked worktrees
        let common_path = git_directory.join(GIT_COMMON_DIRECTORY_FILE_NAME);
        let common_directory = match read_optional(file_system, &common_path)? {
            Some(path) if !path.trim().is_empty() => git_directory.normalize_with(path.trim()),
            Some(_) => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("empty Git common directory: {}", common_path.display()),
                ));
            }
            None => git_directory,
        };

        // prepend repository-local exclusions below .gitignore precedence
        let exclude_path = common_directory.join(GIT_EXCLUDE_PATH);
        let Some(text) = read_optional(file_system, &exclude_path)? else {
            return Ok(());
        };
        let Some(ignore_file) = self.loaded.get_mut(repository_root) else {
            return Err(io::Error::other(format!(
                "repository ignore rules were not loaded: {}",
                repository_root.display()
            )));
        };
        let mut patterns = parse_patterns(&text);
        patterns.append(&mut ignore_file.patterns);
        ignore_file.patterns = patterns;

        Ok(())
    }

    /// Load ignore files for a directory if not already loaded.
    pub fn load(&mut self, file_system: &dyn FileSystem, directory: &Path) -> io::Result<()> {
        let key = directory.to_path_buf();
        if self.loaded.contains_key(&key) {
            return Ok(());
        }

        // load gitignore patterns first
        let mut patterns = Vec::new();
        let ignore_file = directory.join(GITIGNORE_FILE_NAME);
        if let Some(text) = read_optional(file_system, &ignore_file)? {
            patterns.extend(parse_patterns(&text));
        }

        // load stats relevant gitattributes patterns
        let attributes_file = directory.join(GITATTRIBUTES_FILE_NAME);
        if let Some(text) = read_optional(file_system, &attributes_file)? {
            patterns.extend(parse_gitattributes_patterns(&text));
        }

        self.loaded.insert(
            key,
            IgnoreFile {
                base: directory.to_path_buf(),
                patterns,
            },
        );

        Ok(())
    }

    /// Check if a path is ignored, considering all ancestor ignore files.
    pub fn is_ignored(&self, path: &Path, is_directory: bool) -> bool {
        // keep Git control paths outside every source walk
        if path
            .file_name()
            .is_some_and(|name| name == GIT_DIRECTORY_NAME)
        {
            return true;
        }

        // select the first match in Git precedence order
        let ancestors = path.parent().into_iter().flat_map(Path::ancestors);
        for base in ancestors {
            if let Some(ignore_file) = self.loaded.get(base) {
                let relative_path = path
                    .strip_prefix(base)
                    .expect("ignore file base must be a path ancestor");
                let relative_path_string = unix_path(relative_path);
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                for pattern in ignore_file.patterns.iter().rev() {
                    if pattern.directory_only && !is_directory {
                        continue;
                    }
                    if pattern_matches(&pattern.pattern, &relative_path_string, file_name) {
                        return !pattern.is_negation;
                    }
                }
            }
        }

        false
    }
}

/// Read one ignore file when it exists.
fn read_optional(file_system: &dyn FileSystem, path: &Path) -> io::Result<Option<String>> {
    match file_system.read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io::Error::new(
            error.kind(),
            format!("failed to read {}: {error}", path.display()),
        )),
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
        matches(pattern.as_bytes(), relative_path.as_bytes())
    } else {
        matches(pattern.as_bytes(), file_name.as_bytes())
    }
}

/// Create a unix style path string with forward slashes.
fn unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
        fs.create_dir_all(Path::new("project/src"))
            .expect("create ignore directory");
        fs.create_dir_all(Path::new("meta/worktrees/project"))
            .expect("create worktree directory");
        fs.create_dir_all(Path::new("meta/info"))
            .expect("create Git directory");
        let ignore_file = r"target/
*.log
!keep.log
# comment
";
        fs.write_bytes("project/.gitignore", ignore_file.as_bytes())
            .expect("write ignore file");
        fs.write_bytes("project/.git", b"gitdir: ../meta/worktrees/project\n")
            .expect("write Git directory file");
        fs.write_bytes("meta/worktrees/project/commondir", b"../..\n")
            .expect("write common directory file");
        fs.write_bytes("meta/info/exclude", b"*.log\n**/.claude/worktrees/\n")
            .expect("write repository exclude file");

        let mut ignore_set = IgnoreSet::new();
        ignore_set
            .load_root(&fs, &fs.path_for("project/src"))
            .expect("load ignore rules");

        let target_file = fs.path_for("project/src/target");
        assert!(ignore_set.is_ignored(&target_file, true));

        let log_file = fs.path_for("project/src/foo.log");
        assert!(ignore_set.is_ignored(&log_file, false));

        let keep_file = fs.path_for("project/src/keep.log");
        assert!(!ignore_set.is_ignored(&keep_file, false));

        let worktree = fs.path_for("project/src/.claude/worktrees");
        assert!(ignore_set.is_ignored(&worktree, true));

        let git_directory = fs.path_for("project/.git");
        assert!(ignore_set.is_ignored(&git_directory, true));
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
        fs.write_bytes("project/.gitattributes", attributes_file.as_bytes())
            .expect("write attributes file");

        let mut ignore_set = IgnoreSet::new();
        ignore_set
            .load(&fs, &fs.path_for("project"))
            .expect("load attribute rules");

        let generated_file = fs.path_for("project/generated/parser.c");
        assert!(ignore_set.is_ignored(&generated_file, false));

        let kept_file = fs.path_for("project/generated/keep.c");
        assert!(!ignore_set.is_ignored(&kept_file, false));

        let generated_rust_file = fs.path_for("project/runtime/bindings.generated.rs");
        assert!(ignore_set.is_ignored(&generated_rust_file, false));

        let vendor_file = fs.path_for("project/vendor.txt");
        assert!(ignore_set.is_ignored(&vendor_file, false));

        let archive_file = fs.path_for("project/archive.tar");
        assert!(ignore_set.is_ignored(&archive_file, false));

        let source_file = fs.path_for("project/source.rs");
        assert!(!ignore_set.is_ignored(&source_file, false));
    }
}
