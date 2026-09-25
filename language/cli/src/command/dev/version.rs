use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use clap::Subcommand;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use destack_source::glob;

use crate::console;

const FILE_GLOBS_TO_UPDATE: &[&str] = &[
    "README.md",
    "Cargo.toml",
    "package.json",
    "language/library/destack.json",
    "language/grammar/destack/tree-sitter.json",
    "language/grammar/bytecode/tree-sitter.json",
    "language/grammar/mir/tree-sitter.json",
    "language/bridge/zed/Cargo.toml",
    "language/bridge/zed/extension.toml",
    "*/package.json",
    "*/*/package.json",
    "*/*/*/package.json",
    "*/*/*/*/package.json",
];
const FILE_GLOBS_TO_IGNORE: &[&str] = &[
    "language/formatter/fixture/",
    "language/query/fixture/",
    "language/bridge/fixture/",
    "language/bridge/zed/grammars/",
];
const MANIFEST_PATH: &str = "package.json";

/// Calendar version with `year.month.micro` format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    /// Gregorian calendar year.
    year: u16,
    /// Gregorian calendar month.
    month: u8,
    /// Release number within the month.
    micro: u16,
}

impl Version {
    /// Parse one canonical calendar version.
    fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let mut components = text.split('.');
        let year = Self::parse_component(components.next(), "year")?;
        let month = Self::parse_component(components.next(), "month")?;
        let micro = Self::parse_component(components.next(), "micro")?;

        // reject additional components
        if components.next().is_some() {
            return Err(format!(
                "invalid version '{text}': expected YEAR.MONTH.MICRO"
            ));
        }

        // require the canonical calendar year width
        if !(1000..=9999).contains(&year) {
            return Err(format!(
                "invalid version '{text}': year must contain four digits"
            ));
        }

        // require one Gregorian calendar month
        if !(1..=12).contains(&month) {
            return Err(format!(
                "invalid version '{text}': month must be between 1 and 12"
            ));
        }

        let version = Self { year, month, micro };

        // reject leading zeros and other noncanonical representations
        if version.to_string() != text {
            return Err(format!(
                "invalid version '{text}': expected canonical {version}"
            ));
        }

        Ok(version)
    }

    /// Return the next release version for one calendar month.
    fn next(self, year: u16, month: u8) -> Result<Self, String> {
        let current_period = (self.year, self.month);
        let next_period = (year, month);

        // reject release months before the current version
        if next_period < current_period {
            return Err(format!(
                "current version {self} is later than release month {year}.{month}"
            ));
        }

        // advance within the current release month
        if next_period == current_period {
            return Ok(Self {
                micro: self.micro + 1,
                ..self
            });
        }

        // begin a new monthly sequence
        Ok(Self {
            year,
            month,
            micro: 0,
        })
    }

    /// Read the canonical repository version.
    fn read() -> Result<Self, String> {
        let text = read_version()?;

        Self::parse(&text)
    }

    /// Parse one numeric version component.
    fn parse_component<T>(component: Option<&str>, name: &str) -> Result<T, String>
    where
        T: std::str::FromStr,
    {
        let component = component.ok_or_else(|| format!("missing version {name}"))?;

        component
            .parse()
            .map_err(|_| format!("invalid version {name} '{component}'"))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.year, self.month, self.micro)
    }
}

/// Repository version commands.
#[derive(Subcommand, Clone, Debug)]
pub enum VersionCommand {
    /// Advance to the next release version in the current UTC month.
    Next,
    /// Set one explicit release version.
    Set {
        /// Calendar version in `YEAR.MONTH.MICRO` format.
        version: String,
    },
    /// Check tracked files for version drift.
    Check,
    /// Show the current version.
    Show,
}

impl VersionCommand {
    /// Run this version command.
    pub fn run(&self) -> i32 {
        let result = match self {
            Self::Next => next_version(),
            Self::Set { version } => Version::parse(version).and_then(set_version),
            Self::Check => check_version(),
            Self::Show => show_version(),
        };

        // report command failures through the CLI
        match result {
            Ok(()) => 0,
            Err(error) => {
                console::error(&format!("error: {error}"));

                1
            }
        }
    }
}

/// Advance the repository version in the current UTC month.
fn next_version() -> Result<(), String> {
    let date = Timestamp::now().to_zoned(TimeZone::UTC).date();
    let year = date.year() as u16;
    let month = date.month() as u8;
    let version = Version::read()?.next(year, month)?;

    set_version(version)
}

/// Set the repository version in every tracked file.
fn set_version(version: Version) -> Result<(), String> {
    let current = read_version()?;
    let paths = collect_tracked_paths()?;
    let mut updates = Vec::with_capacity(paths.len());

    // prepare every update before changing the working tree
    for path in paths {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let updated = replace_version(&path, &text, &current, version)?;

        updates.push((path, updated));
    }

    // write the complete prepared update
    for (path, updated) in updates {
        fs::write(&path, updated)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }

    console::print(&format!("Version: {current} -> {version}"));

    Ok(())
}

/// Check every tracked file against the canonical repository version.
fn check_version() -> Result<(), String> {
    let version = Version::read()?;
    let current = version.to_string();

    // require every tracked file to contain the canonical version
    for path in collect_tracked_paths()? {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        replace_version(&path, &text, &current, version)?;
    }

    console::print(&format!("Version: {version}"));

    Ok(())
}

/// Show the canonical repository version.
fn show_version() -> Result<(), String> {
    let version = Version::read()?;
    console::print(&version.to_string());

    Ok(())
}

/// Read the repository version from its workspace manifest.
fn read_version() -> Result<String, String> {
    let text = fs::read_to_string(MANIFEST_PATH)
        .map_err(|error| format!("failed to read {MANIFEST_PATH}: {error}"))?;
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {MANIFEST_PATH}: {error}"))?;
    let version = manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{MANIFEST_PATH} does not declare a string version"))?;

    Ok(version.to_string())
}

/// Collect tracked version paths without duplicates.
fn collect_tracked_paths() -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    // resolve configured paths in declaration order
    for configured in FILE_GLOBS_TO_UPDATE {
        let resolved = if configured.contains('*') {
            glob(configured)
                .map_err(|error| format!("failed to expand version path {configured}: {error}"))?
        } else {
            vec![PathBuf::from(configured)]
        };

        // retain each resolved file once
        for path in resolved {
            if should_ignore_path(&path) || !seen.insert(path.clone()) {
                continue;
            }

            paths.push(path);
        }
    }

    // reject an empty repository selection
    if paths.is_empty() {
        return Err("no tracked version files found".to_string());
    }

    Ok(paths)
}

/// Return whether one path is excluded from version updates.
fn should_ignore_path(path: &Path) -> bool {
    FILE_GLOBS_TO_IGNORE
        .iter()
        .any(|ignored| path.to_string_lossy().contains(ignored))
}

/// Replace the exact repository version in one tracked file.
fn replace_version(
    path: &Path,
    text: &str,
    current: &str,
    version: Version,
) -> Result<String, String> {
    // require ordinary tracked files to contain the repository version
    if !text.contains(current) {
        return Err(format!(
            "{} does not contain canonical version {current}",
            path.display()
        ));
    }

    Ok(text.replace(current, &version.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calendar versions parse, render, and advance by release month.
    #[test]
    fn test_advance_version() {
        let version = Version::parse("2026.8.2").unwrap();

        assert_eq!(version.to_string(), "2026.8.2");
        assert_eq!(version.next(2026, 8).unwrap().to_string(), "2026.8.3");
        assert_eq!(version.next(2026, 9).unwrap().to_string(), "2026.9.0");
        assert_eq!(
            version.next(2026, 7).unwrap_err(),
            "current version 2026.8.2 is later than release month 2026.7"
        );
    }
}
