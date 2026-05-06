use std::path::{Component, Path, PathBuf};

use destack_source::PathExt;

/// Compute a relative path between two paths when possible.
pub(crate) fn relative_path(from_dir: &Path, to_path: &Path) -> Option<PathBuf> {
    // normalize input paths
    let from_dir = from_dir.normalize();
    let to_path = to_path.normalize();

    // collect normalized path components
    let from_components = normal_components(&from_dir);
    let to_components = normal_components(&to_path);

    // find the shared prefix length
    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    // refuse to compute across unrelated absolute roots
    if common == 0 && from_dir.is_absolute() && to_path.is_absolute() {
        return None;
    }

    // build the relative path segments
    let mut relative = PathBuf::new();
    // add parent segments for the shared prefix
    for _ in common..from_components.len() {
        relative.push("..");
    }

    // append the remaining target components
    for component in &to_components[common..] {
        relative.push(component);
    }

    // return the computed relative path
    Some(relative)
}

/// Normalize path separators to forward slashes.
pub(crate) fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Collect normalized path components for stable comparisons.
fn normal_components(path: &Path) -> Vec<String> {
    // collect normalized components
    let mut components = Vec::new();

    // translate platform specific components into normalized strings
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => {
                components.push("/".to_string());
            }
            Component::Normal(part) => {
                components.push(part.to_string_lossy().to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                components.push("..".to_string());
            }
        }
    }

    // return the normalized components
    components
}
