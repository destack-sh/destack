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

/// Compute a heuristic distance between two paths.
pub(crate) fn path_distance(from_dir: &Path, to_path: &Path) -> u32 {
    // normalize the directories before comparing components
    let from_dir = from_dir.normalize();
    let to_dir = to_path.parent().unwrap_or(to_path).normalize();

    // resolve components for both paths
    let from_components = normal_components(&from_dir);
    let to_components = normal_components(&to_dir);

    // compute the shared prefix length
    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    // compute the number of path steps
    let ups = from_components.len().saturating_sub(common);
    let downs = to_components.len().saturating_sub(common);

    // return the computed distance
    (ups + downs) as u32
}

/// Count normalized components for a path.
pub(crate) fn path_component_count(path: &Path) -> u32 {
    normal_components(path).len() as u32
}

/// Normalize path separators to forward slashes.
pub(crate) fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Return the number of shared suffix components between two paths.
pub(crate) fn common_suffix_len(left: &Path, right: &Path) -> usize {
    let left_components: Vec<_> = left.components().collect();
    let right_components: Vec<_> = right.components().collect();
    let mut count = 0usize;

    while count < left_components.len() && count < right_components.len() {
        let left_idx = left_components.len() - 1 - count;
        let right_idx = right_components.len() - 1 - count;
        if left_components[left_idx] != right_components[right_idx] {
            break;
        }
        count += 1;
    }

    count
}

/// Rewrite a path by replacing the shared suffix with the new path suffix.
pub(crate) fn rewrite_path_with_common_suffix(
    specifier_path: &Path,
    old_path: &Path,
    new_path: &Path,
) -> PathBuf {
    let common_len = common_suffix_len(specifier_path, old_path);
    if common_len == 0 {
        return new_path.to_path_buf();
    }

    let suffix = path_suffix(specifier_path, common_len);
    let prefix = strip_path_suffix(specifier_path, &suffix).unwrap_or_default();
    let new_suffix = path_suffix(new_path, common_len);

    if prefix.as_os_str().is_empty() {
        new_suffix
    } else {
        prefix.join(new_suffix)
    }
}

/// Collect the last N components from a path.
fn path_suffix(path: &Path, component_count: usize) -> PathBuf {
    let components: Vec<_> = path.components().collect();
    let start = components.len().saturating_sub(component_count);
    let mut suffix = PathBuf::new();
    for component in &components[start..] {
        suffix.push(component.as_os_str());
    }

    suffix
}

/// Strip a path suffix, returning the prefix path.
pub(crate) fn strip_path_suffix(path: &Path, suffix: &Path) -> Option<PathBuf> {
    let path_components: Vec<_> = path.components().collect();
    let suffix_components: Vec<_> = suffix.components().collect();
    if suffix_components.len() > path_components.len() {
        return None;
    }

    let split_at = path_components.len() - suffix_components.len();
    if path_components[split_at..] != suffix_components[..] {
        return None;
    }

    let mut prefix = PathBuf::new();
    for component in &path_components[..split_at] {
        prefix.push(component.as_os_str());
    }

    Some(prefix)
}

/// Strip the extension from a path when it has one.
pub(crate) fn strip_path_extension(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?;
    let stem = path.file_stem()?;
    if file_name == stem {
        return None;
    }

    let parent = path.parent()?;
    Some(parent.join(stem))
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
