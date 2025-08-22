//! Version bump CLI ported from Python.

use crate::console::console;
use crate::console::parse::{CommandApp, CommandArguments};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const FILES_TO_UPDATE: &[&str] = &[
    "version.txt",
    "pyproject.toml",
    "package.json",
    "Cargo.toml",
    "client/destack_py/python/pyproject.toml",
    "ext/vscode/package.json",
    "ext/zed/Cargo.toml",
    "ext/zed/extension.toml",
];

/// Create the version command app.
pub fn app() -> CommandApp {
    CommandApp::new("version")
        .help("Mark new versions.")
        .command(
            "bump",
            bump,
            Some("Bump CalVer (YYYY.MM.DD.R).".to_string()),
        )
}

/// Bump the version using CalVer format.
///
/// Increments the revision number if the date is the same, otherwise resets to 0.
/// Updates all relevant files with the new version.
fn bump(ctx: CommandArguments) -> i32 {
    let override_rev: Option<i32> = ctx.option("revision").and_then(|s| s.parse::<i32>().ok());

    let current_version =
        read_current_version().unwrap_or_else(|| panic!("version file not found"));

    // extract date and revision from current version
    let current_date = current_version
        .split('.')
        .take(3)
        .collect::<Vec<_>>()
        .join(".");
    let current_rev: i32 = current_version
        .split('.')
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let current_semver = to_semver(&current_version);

    // determine new version
    let today = today_calver();
    let new_rev = match override_rev {
        Some(r) => r,
        None => {
            if today == current_date {
                current_rev + 1
            } else {
                0
            }
        }
    };
    let new_version = format!("{today}.{new_rev}");
    let new_semver = to_semver(&new_version);

    console::print(&format!("Version: {current_version} -> {new_version}"));

    // read and validate all files before making changes
    let mut contents: Vec<(PathBuf, String)> = Vec::new();
    for rel in FILES_TO_UPDATE {
        let p = PathBuf::from(rel);
        match fs::read_to_string(&p) {
            Ok(text) => {
                if !text.contains(&current_version) && !text.contains(&current_semver) {
                    console::error(&format!(
                        "{current_version} or {current_semver} not found in {rel}"
                    ));
                    return 1;
                }
                contents.push((p, text));
            }
            Err(e) => {
                console::error(&format!("{} not found ({e})", p.display()));
                return 1;
            }
        }
    }

    // write new version to version file
    if let Err(e) = fs::write("version.txt", &new_version) {
        console::error(&format!("failed to write version: {e}"));
        return 1;
    }

    // update all files with new version
    for (p, text) in contents.into_iter() {
        let updated = text
            .replace(&current_version, &new_version)
            .replace(&current_semver, &new_semver);
        if let Err(e) = fs::write(&p, updated) {
            console::error(&format!("failed to write {}: {e}", p.display()));
            return 1;
        }
    }

    0
}

/// Convert CalVer format to SemVer format.
///
/// Transforms YYYY.MM.DD.R to Y.M.D-R by removing leading zeros.
fn to_semver(calver: &str) -> String {
    let parts: Vec<&str> = calver.split('.').collect();
    if parts.len() != 4 {
        return calver.to_string();
    }
    let year = parts[0].trim_start_matches('0');
    let year = if year.is_empty() { "0" } else { year };
    let month = parts[1].trim_start_matches('0');
    let month = if month.is_empty() { "0" } else { month };
    let day = parts[2].trim_start_matches('0');
    let day = if day.is_empty() { "0" } else { day };
    let rev = parts[3];
    format!("{year}.{month}.{day}-{rev}")
}

/// Read the current version from the version file.
fn read_current_version() -> Option<String> {
    fs::read_to_string("version.txt")
        .ok()
        .map(|s| s.trim().to_string())
}

/// Get today's date in CalVer format (YYYY.MM.DD).
fn today_calver() -> String {
    let out = Command::new("date").arg("+%Y.%m.%d").output();
    if let Ok(o) = out
        && o.status.success()
        && let Ok(s) = String::from_utf8(o.stdout)
    {
        return s.trim().to_string();
    }
    panic!("failed to get today's date");
}
