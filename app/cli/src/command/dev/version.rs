use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use clap::Subcommand;
use destack_source::glob;

use crate::console;

const FILE_GLOBS_TO_UPDATE: &[&str] = &[
    "VERSION.txt",
    "README.md",
    "Cargo.toml",
    "package.json",
    "bridge/typescript/src/index.ts",
    "bridge/rust/README.md",
    "bridge/rust/aliases/destack-rs/README.md",
    "bridge/python/README.md",
    "bridge/python/src/destack/client.py",
    "bridge/python/pyproject.toml",
    "bridge/python/aliases/destack-py/pyproject.toml",
    "bridge/rust/aliases/destack-rs/Cargo.toml",
    "bridge/zed/extension.toml",
    "*/package.json",
    "*/*/package.json",
    "*/*/*/package.json",
    "*/*/*/*/package.json",
];
const FILE_GLOBS_TO_IGNORE: &[&str] = &[
    "node_modules/",
    "template/create-destack/templates/",
    "language/test/fixtures/",
    "language/grammar/destack/",
    "bridge/zed/grammars/",
];
const ROOT_README_PATH: &str = "README.md";
const README_VERSION_BADGE_PREFIX: &str = "https://img.shields.io/badge/version-";
const README_VERSION_BADGE_SUFFIX: &str = "-2ea44f";

#[derive(Subcommand, Clone, Debug)]
pub enum VersionCommands {
    /// Bump major version (X.0.0).
    Major,
    /// Bump minor version (x.Y.0).
    Minor,
    /// Bump patch version (x.y.Z).
    Patch,
    /// Check tracked files for version drift.
    Check,
    /// Show current version.
    Show,
}

/// Semantic version with major.minor.patch format.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVer {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemVer {
    fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    fn bump_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
        }
    }

    fn bump_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    fn bump_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Show the current version.
pub fn show() -> i32 {
    match read_current_version() {
        Some(v) => {
            console::print(&v);
            0
        }
        None => {
            console::error("version file not found");
            1
        }
    }
}

/// Bump the version using SemVer format.
///
/// Updates all relevant files with the new version.
pub fn bump(kind: &VersionCommands) -> i32 {
    let current_str = read_current_version().unwrap_or_else(|| panic!("version file not found"));
    let current = SemVer::parse(&current_str)
        .unwrap_or_else(|| panic!("invalid version format: {current_str} (expected X.Y.Z)"));

    let new = match kind {
        VersionCommands::Major => current.bump_major(),
        VersionCommands::Minor => current.bump_minor(),
        VersionCommands::Patch => current.bump_patch(),
        VersionCommands::Check | VersionCommands::Show => unreachable!(),
    };

    let current_version = current.to_string();
    let new_version = new.to_string();

    console::print(&format!("Version: {current_version} -> {new_version}"));
    console::print(&"=".repeat(80));

    // collect all version tracked files
    let tracked_paths = match collect_tracked_paths() {
        Ok(paths) => paths,
        Err(error) => {
            console::error(&format!("error: {error}"));
            return 1;
        }
    };

    // read and validate all files before making changes
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for path in tracked_paths {
        console::print(&format!("  {}", path.display()));
        match fs::read_to_string(&path) {
            Ok(text) => {
                let updated =
                    match update_file_version(&path, &text, &current_version, &new_version) {
                        Ok(updated) => updated,
                        Err(e) => {
                            console::error(&format!("error: {e} in {path:?}"));
                            return 1;
                        }
                    };
                files.push((path, updated));
            }
            Err(e) => {
                console::error(&format!("error: {path:?} not found ({e})"));
                return 1;
            }
        }
    }

    // write all updated files
    for (p, updated) in files.into_iter() {
        if let Err(e) = fs::write(&p, updated) {
            console::error(&format!("error: failed to write {} ({e})", p.display()));
            return 1;
        }
    }

    0
}

/// Check tracked files for version drift.
pub fn check() -> i32 {
    let current_version =
        read_current_version().unwrap_or_else(|| panic!("version file not found"));
    SemVer::parse(&current_version)
        .unwrap_or_else(|| panic!("invalid version format: {current_version} (expected X.Y.Z)"));

    console::print(&format!("Version check: {current_version}"));
    console::print(&"=".repeat(80));

    // collect all version tracked files
    let tracked_paths = match collect_tracked_paths() {
        Ok(paths) => paths,
        Err(error) => {
            console::error(&format!("error: {error}"));
            return 1;
        }
    };

    // collect every mismatch to avoid multiple ci reruns
    let mut mismatch_count = 0;
    for path in tracked_paths {
        console::print(&format!("  {}", path.display()));

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                console::error(&format!("error: {path:?} not found ({error})"));
                mismatch_count += 1;
                continue;
            }
        };

        match update_file_version(&path, &text, &current_version, &current_version) {
            Ok(updated) => {
                if updated != text {
                    console::error(&format!(
                        "error: {} has version drift from {}",
                        path.display(),
                        current_version
                    ));
                    mismatch_count += 1;
                }
            }
            Err(error) => {
                console::error(&format!("error: {error} in {path:?}"));
                mismatch_count += 1;
            }
        }
    }

    if mismatch_count > 0 {
        console::error(&format!("found {mismatch_count} version drift errors"));
        return 1;
    }

    console::print("Version check passed");
    0
}

/// Return whether a path should be excluded from bulk version updates.
fn should_ignore_path(path: &Path) -> bool {
    FILE_GLOBS_TO_IGNORE
        .iter()
        .any(|ignore| path.to_string_lossy().contains(ignore))
}

/// Collect tracked version file paths without duplicates.
fn collect_tracked_paths() -> Result<Vec<PathBuf>, String> {
    let mut tracked_paths: Vec<PathBuf> = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    // resolve each configured glob and keep first match ordering
    for glob_path in FILE_GLOBS_TO_UPDATE {
        let resolved_paths = if glob_path.contains('*') {
            glob(glob_path)
        } else {
            vec![PathBuf::from(glob_path)]
        };

        for path in resolved_paths {
            if should_ignore_path(&path) {
                continue;
            }

            let path_key = path.to_string_lossy().to_string();
            if !seen_paths.insert(path_key) {
                continue;
            }

            tracked_paths.push(path);
        }
    }

    if tracked_paths.is_empty() {
        return Err("no tracked files resolved from FILE_GLOBS_TO_UPDATE".to_string());
    }

    Ok(tracked_paths)
}

/// Update a file with the new version.
///
/// Falls back to the README badge version when the root README drifted.
fn update_file_version(
    path: &Path,
    text: &str,
    current_version: &str,
    new_version: &str,
) -> Result<String, String> {
    // normal path: update exact version matches
    if text.contains(current_version) {
        return Ok(text.replace(current_version, new_version));
    }

    // fallback: recover root README badge drift
    if path == Path::new(ROOT_README_PATH) {
        let Some(updated) = update_readme_badge_version(text, new_version) else {
            return Err(format!(
                "{current_version} not found and README version badge was not parseable"
            ));
        };
        return Ok(updated);
    }

    Err(format!("{current_version} not found"))
}

/// Update the root README badge version.
fn update_readme_badge_version(text: &str, new_version: &str) -> Option<String> {
    let badge_prefix_start = text.find(README_VERSION_BADGE_PREFIX)?;
    let version_start = badge_prefix_start + README_VERSION_BADGE_PREFIX.len();

    let badge_suffix_start = text[version_start..].find(README_VERSION_BADGE_SUFFIX)?;
    let version_end = version_start + badge_suffix_start;
    let current_badge_version = &text[version_start..version_end];

    SemVer::parse(current_badge_version)?;

    let mut updated = String::with_capacity(text.len() + new_version.len());
    updated.push_str(&text[..version_start]);
    updated.push_str(new_version);
    updated.push_str(&text[version_end..]);

    Some(updated)
}

/// Read the current version from the version file.
fn read_current_version() -> Option<String> {
    fs::read_to_string("VERSION.txt")
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_parse_valid_semver() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_parse_invalid_semver() {
        assert!(SemVer::parse("1.2").is_none());
        assert!(SemVer::parse("1.2.3.4").is_none());
        assert!(SemVer::parse("a.b.c").is_none());
    }

    #[test]
    fn test_bump_major() {
        let v = SemVer::parse("1.2.3").unwrap();
        let bumped = v.bump_major();
        assert_eq!(bumped.to_string(), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        let v = SemVer::parse("1.2.3").unwrap();
        let bumped = v.bump_minor();
        assert_eq!(bumped.to_string(), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let v = SemVer::parse("1.2.3").unwrap();
        let bumped = v.bump_patch();
        assert_eq!(bumped.to_string(), "1.2.4");
    }

    #[test]
    fn test_display() {
        let v = SemVer {
            major: 0,
            minor: 2,
            patch: 1,
        };
        assert_eq!(v.to_string(), "0.2.1");
    }

    #[test]
    fn test_update_file_version_replaces_current_version() {
        let path = Path::new("Cargo.toml");
        let source = "version = \"0.55.2\"";
        let updated = update_file_version(path, source, "0.55.2", "0.55.3").unwrap();

        assert_eq!(updated, "version = \"0.55.3\"");
    }

    #[test]
    fn test_update_file_version_recovers_readme_badge_drift() {
        let path = Path::new("README.md");
        let source =
            "<img src=\"https://img.shields.io/badge/version-0.55.1-2ea44f\" alt=\"Version\">";
        let updated = update_file_version(path, source, "0.55.2", "0.55.3").unwrap();

        assert_eq!(
            updated,
            "<img src=\"https://img.shields.io/badge/version-0.55.3-2ea44f\" alt=\"Version\">"
        );
    }

    #[test]
    fn test_update_file_version_errors_without_matching_version() {
        let path = Path::new("Cargo.toml");
        let source = "version = \"0.55.1\"";
        let error = update_file_version(path, source, "0.55.2", "0.55.3").unwrap_err();

        assert_eq!(error, "0.55.2 not found");
    }

    #[test]
    fn test_update_file_version_check_mode_keeps_matching_text() {
        let path = Path::new("Cargo.toml");
        let source = "version = \"0.55.3\"";
        let updated = update_file_version(path, source, "0.55.3", "0.55.3").unwrap();

        assert_eq!(updated, source);
    }

    #[test]
    fn test_update_file_version_check_mode_detects_readme_badge_drift() {
        let path = Path::new("README.md");
        let source =
            "<img src=\"https://img.shields.io/badge/version-0.55.1-2ea44f\" alt=\"Version\">";
        let updated = update_file_version(path, source, "0.55.3", "0.55.3").unwrap();

        assert_ne!(updated, source);
    }

    #[test]
    fn test_should_ignore_path_for_node_modules() {
        let path = Path::new("node_modules/agent-base/package.json");

        assert!(should_ignore_path(path));
    }

    #[test]
    fn test_should_not_ignore_path_for_workspace_package() {
        let path = Path::new("bridge/vscode/package.json");

        assert!(!should_ignore_path(path));
    }

    #[test]
    fn test_should_ignore_path_for_project_template_package() {
        let path = Path::new("template/create-destack/templates/blank/package.json");

        assert!(should_ignore_path(path));
    }
}
