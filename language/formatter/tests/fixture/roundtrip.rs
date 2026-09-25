use std::path::{Path, PathBuf};

use libtest_mimic::{Failed, Trial};
use tspp_repository::FormatterOptions;
use tspp_source::{DiagnosticSeverity, DiffOptions, format_diff};

use crate::format::format_source;

/// Discover every formatter roundtrip as a test trial.
pub(super) fn trials(directory: &Path) -> Result<Vec<Trial>, String> {
    let mut paths = std::fs::read_dir(directory)
        .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
    paths.retain(|path| path.is_file() && is_source_file(path));
    paths.sort();

    paths.into_iter().map(trial).collect()
}

/// Return whether one path is a TS++ source fixture.
fn is_source_file(path: &Path) -> bool {
    let path = path.to_string_lossy();

    path.ends_with(".tspp") || path.ends_with(".d.tspp")
}

/// Build one roundtrip trial.
fn trial(path: PathBuf) -> Result<Trial, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("fixture path '{}' has no UTF-8 file name", path.display()))?;
    let name = format!("roundtrip/{name}");

    Ok(Trial::test(name, move || run(&path)))
}

/// Verify one canonical source file remains unchanged.
fn run(path: &Path) -> Result<(), Failed> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixture/roundtrip");
    let logical_path = path.strip_prefix(&root).map_err(|error| {
        Failed::from(format!(
            "fixture '{}' is outside '{}': {error}",
            path.display(),
            root.display()
        ))
    })?;
    let original = std::fs::read_to_string(path)
        .map_err(|error| Failed::from(format!("failed to read '{}': {error}", path.display())))?;
    let formatted = format_source(
        logical_path,
        Some(path),
        original.clone(),
        FormatterOptions::default(),
        DiagnosticSeverity::Error,
    )
    .map_err(Failed::from)?;
    if formatted == original {
        return Ok(());
    }

    // bless canonical files only when explicitly requested
    if std::env::var_os("TSPP_BLESS").is_some_and(|value| !value.is_empty() && value != "0") {
        std::fs::write(path, formatted).map_err(|error| {
            Failed::from(format!("failed to update '{}': {error}", path.display()))
        })?;

        return Ok(());
    }
    let difference = format_diff(&original, &formatted, &DiffOptions::new());

    Err(Failed::from(format!(
        "formatted output differs\n\n{difference}"
    )))
}
