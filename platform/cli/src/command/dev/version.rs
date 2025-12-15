use std::path::PathBuf;
use std::{fmt, fs};

use clap::Subcommand;
use destack_source::glob;

use crate::console;

const FILE_GLOBS_TO_UPDATE: &[&str] = &[
    "version.txt",
    "Cargo.toml",
    "package.json",
    "*/package.json",
    "*/*/package.json",
    "*/*/*/package.json",
];
const FILE_GLOBS_TO_IGNORE: &[&str] = &["language/test/fixtures/"];

#[derive(Subcommand, Clone, Debug)]
pub enum VersionCommands {
    /// Bump major version (X.0.0).
    Major,
    /// Bump minor version (x.Y.0).
    Minor,
    /// Bump patch version (x.y.Z).
    Patch,
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
        VersionCommands::Show => unreachable!(),
    };

    let current_version = current.to_string();
    let new_version = new.to_string();

    console::print(&format!("Version: {current_version} -> {new_version}"));
    console::print(&"=".repeat(80));

    // read and validate all files before making changes
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for glob_path in FILE_GLOBS_TO_UPDATE {
        let paths = if glob_path.contains('*') {
            glob(glob_path)
        } else {
            vec![PathBuf::from(glob_path)]
        };
        for path in paths {
            if FILE_GLOBS_TO_IGNORE
                .iter()
                .any(|ignore| path.to_string_lossy().contains(ignore))
            {
                continue;
            }
            console::print(&format!("  {}", path.display()));
            match fs::read_to_string(&path) {
                Ok(text) => {
                    if !text.contains(&current_version) {
                        console::error(&format!("error: {current_version} not found in {path:?}"));
                        return 1;
                    }
                    files.push((path, text));
                }
                Err(e) => {
                    console::error(&format!("error: {path:?} not found ({e})"));
                    return 1;
                }
            }
        }
    }

    // update all files with new version
    for (p, text) in files.into_iter() {
        let updated = text.replace(&current_version, &new_version);
        if let Err(e) = fs::write(&p, updated) {
            console::error(&format!("error: failed to write {} ({e})", p.display()));
            return 1;
        }
    }

    0
}

/// Read the current version from the version file.
fn read_current_version() -> Option<String> {
    fs::read_to_string("version.txt")
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
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
}
