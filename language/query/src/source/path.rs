use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use tspp_source::PathExt;

use crate::{QueryError, QueryResult};

/// Return a normalized relative path between two filesystem locations.
pub(crate) fn relative_path(from_directory: &Path, to_path: &Path) -> Option<PathBuf> {
    let from_directory = from_directory.normalize();
    let to_path = to_path.normalize();
    let from_components = path_components(&from_directory);
    let to_components = path_components(&to_path);

    // find the shared filesystem prefix
    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }
    if common == 0 && from_directory.is_absolute() && to_path.is_absolute() {
        return None;
    }

    // build the relative path
    let mut relative = PathBuf::new();
    for _ in common..from_components.len() {
        relative.push("..");
    }
    for component in &to_components[common..] {
        relative.push(component);
    }

    Some(relative)
}

/// Count directory steps from one directory to a target path.
pub(crate) fn directory_distance(from_directory: &Path, target_path: &Path) -> QueryResult<u32> {
    let target_directory = match target_path.parent() {
        Some(parent) => parent,
        None => target_path,
    };
    let from_components = path_components(from_directory);
    let target_components = path_components(target_directory);

    // find the shared filesystem prefix
    let mut common = 0usize;
    while common < from_components.len()
        && common < target_components.len()
        && from_components[common] == target_components[common]
    {
        common += 1;
    }

    // count upward and downward directory steps
    let steps = from_components.len() - common + target_components.len() - common;

    let distance = u32::try_from(steps)
        .map_err(|_| QueryError::invalid(format!("path component count: {steps:?}")))?;

    Ok(distance)
}

/// Count normalized components in one path.
pub(crate) fn path_depth(path: &Path) -> QueryResult<u32> {
    let depth = path_components(path).len();
    let depth = u32::try_from(depth)
        .map_err(|_| QueryError::invalid(format!("path component count: {depth:?}")))?;

    Ok(depth)
}

/// Return one path with portable source specifier separators.
pub(crate) fn path_text(path: &Path) -> QueryResult<String> {
    let path = path
        .to_str()
        .ok_or_else(|| QueryError::invalid(format!("non-Unicode source path: {path:?}")))?;

    Ok(path.replace('\\', "/"))
}

/// Remove one supported code module extension.
pub(crate) fn strip_module_extension(path: &str) -> String {
    for extension in [".d.tspp", ".tspp"] {
        if let Some(stripped) = path.strip_suffix(extension) {
            return stripped.to_string();
        }
    }

    path.to_string()
}

/// Collect normalized path components for stable comparison.
fn path_components(path: &Path) -> Vec<OsString> {
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_os_string()),
            Component::RootDir => Some(OsString::from("/")),
            Component::Normal(part) => Some(part.to_os_string()),
            Component::ParentDir => Some(OsString::from("..")),
            Component::CurDir => None,
        })
        .collect()
}
