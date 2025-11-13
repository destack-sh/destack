use std::fs;
use std::path::PathBuf;
use std::process::Command;

use dyst_source::glob::glob;

use crate::{CommandApp, CommandArguments, console};

const FILE_GLOBS_TO_UPDATE: &[&str] = &[
    "version.txt",
    "Cargo.toml",
    "package.json",
    "*/package.json",
    "*/*/package.json",
    "*/*/*/package.json",
];

/// Create the version command app.
pub fn app() -> CommandApp {
    CommandApp::new("version")
        .help("Mark new versions.")
        .default_command("bump")
        .command(
            "bump",
            bump,
            Some("Bump CalVer (YYYY.MM.DD.R).".to_string()),
        )
}

/// Bump the version use CalVer format.
///
/// Increments the revision number if the date is the same, otherwise resets to 0.
/// Updates all relevant files with the new version.
fn bump(_ctx: CommandArguments) -> i32 {
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

    // make new version
    let today = today_calver();
    let new_rev = if today == current_date {
        current_rev + 1
    } else {
        0
    };
    let new_version = format!("{today}.{new_rev}");
    let new_semver = to_semver(&new_version);

    console::print(&format!("Version: {current_version} -> {new_version}"));
    console::print(&"=".repeat(100));

    // read and validate all files before making changes
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for glob_path in FILE_GLOBS_TO_UPDATE {
        let paths = {
            if glob_path.contains("*") {
                glob(glob_path)
            } else {
                vec![PathBuf::from(glob_path)]
            }
        };
        for path in paths {
            console::print(&path.to_string_lossy());
            match fs::read_to_string(&path) {
                Ok(text) => {
                    if !text.contains(&current_version) && !text.contains(&current_semver) {
                        console::error(&format!(
                            "{current_version} or {current_semver} not found in {path:?} (from \"{glob_path}\")"
                        ));
                        return 1;
                    }
                    files.push((path, text));
                }
                Err(e) => {
                    console::error(&format!("{path:?} not found: {e} (from \"{glob_path}\")"));
                    return 1;
                }
            }
        }
    }

    // write new version to version file
    if let Err(e) = fs::write("version.txt", &new_version) {
        console::error(&format!("failed to write version: {e}"));
        return 1;
    }

    // update all files with new version
    for (p, text) in files.into_iter() {
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
