use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tspp_source::PathExt;

use crate::source::{path_text, relative_path, strip_module_extension};
use crate::{QueryError, QueryResult};

/// Return one absolute normalized path relative to the workspace root.
pub(crate) fn workspace_path(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.normalize()
    } else {
        workspace_root.join(path).normalize()
    }
}

/// Resolve one exact target rewrite from the most specific renamed ancestor.
pub(crate) fn renamed_target_path(
    target_path: &Path,
    renames: &BTreeMap<PathBuf, PathBuf>,
) -> QueryResult<Option<PathBuf>> {
    let Some((old_path, new_path)) = renames
        .iter()
        .filter(|(old_path, _)| target_path.starts_with(old_path))
        .max_by_key(|(old_path, _)| old_path.components().count())
    else {
        return Ok(None);
    };
    let relative = target_path.strip_prefix(old_path).map_err(|_| {
        QueryError::invalid(format!(
            "rename ancestor {} for {}",
            old_path.display(),
            target_path.display()
        ))
    })?;
    let updated = new_path.join(relative).normalize();

    Ok(Some(updated))
}

/// Rewrite one authored specifier after moving its source or target.
pub(crate) fn rename_specifier(
    source_path: Option<&Path>,
    target_path: &Path,
    specifier: &str,
) -> QueryResult<Option<String>> {
    // relative specifiers follow the source and target paths
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let Some(source_path) = source_path else {
            return Ok(None);
        };
        let strip_extension = strip_module_extension(specifier) == specifier;

        relative_specifier(source_path, target_path, strip_extension)
    }
    // package specifiers retain their declared export path
    else {
        Ok(Some(specifier.to_string()))
    }
}

/// Build one relative authored specifier from a source file to a target.
fn relative_specifier(
    source_path: &Path,
    target_path: &Path,
    strip_extension: bool,
) -> QueryResult<Option<String>> {
    let Some(source_directory) = source_path.parent() else {
        return Ok(None);
    };
    let Some(relative) = relative_path(source_directory, target_path) else {
        return Ok(None);
    };
    let mut display = display_path(&relative, strip_extension)?;

    if !display.starts_with("./") && !display.starts_with("../") {
        display = format!("./{display}");
    }

    Ok(Some(display))
}

/// Return one normalized display path with optional module extension removal.
fn display_path(path: &Path, strip_extension: bool) -> QueryResult<String> {
    let display = path_text(path)?;

    Ok(maybe_strip_extension(display, strip_extension))
}

/// Remove one authored module extension when requested.
fn maybe_strip_extension(path: String, strip_extension: bool) -> String {
    if !strip_extension {
        return path;
    }

    strip_module_extension(&path)
}
