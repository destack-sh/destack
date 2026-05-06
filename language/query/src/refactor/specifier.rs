use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use destack_source::{FileSystem, ModuleSpecifier, PathExt};

/// One shared policy context for specifier matching and rewriting.
#[derive(Debug, Clone, Copy)]
pub(super) struct SpecifierPolicy<'a> {
    /// The filesystem view used for existence checks.
    pub fs: &'a dyn FileSystem,
    /// The workspace root used for relative rename entries.
    pub workspace_root: &'a Path,
}

/// One matched rename target for a specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpecifierRenameMatch {
    /// The matched old target path.
    pub old_path: PathBuf,
    /// The rewritten new target path.
    pub new_path: PathBuf,
    /// The package name when this is one package rewrite.
    pub package_name: Option<String>,
    /// The package root directory when this is one package rewrite.
    pub package_directory: Option<PathBuf>,
}

/// Match one rename entry for a module specifier.
pub(super) fn match_specifier_rename(
    policy: &SpecifierPolicy<'_>,
    rename_map: &HashMap<PathBuf, PathBuf>,
    source_path: Option<&Path>,
    specifier: &str,
    resolved_target_path: Option<&Path>,
    package_name: Option<&str>,
    package_directory: Option<&Path>,
) -> Option<SpecifierRenameMatch> {
    // prefer the resolved module target when one semantic target is known
    if let Some(target_path) = resolved_target_path {
        let target_path = target_path.normalize();

        if let Some((old_path, new_path)) =
            match_path_rename_entry(policy, rename_map, &target_path)
        {
            return Some(SpecifierRenameMatch {
                old_path,
                new_path,
                package_name: package_name.map(ToString::to_string),
                package_directory: package_directory.map(Path::to_path_buf),
            });
        }

        if let Some((old_path, new_path)) =
            match_directory_rename_target(policy, rename_map, &target_path)
        {
            return Some(SpecifierRenameMatch {
                old_path,
                new_path,
                package_name: package_name.map(ToString::to_string),
                package_directory: package_directory.map(Path::to_path_buf),
            });
        }

        if let Some((old_path, new_path)) =
            match_file_uri_rename_entry(policy, rename_map, specifier)
        {
            return Some(SpecifierRenameMatch {
                old_path,
                new_path,
                package_name: package_name.map(ToString::to_string),
                package_directory: package_directory.map(Path::to_path_buf),
            });
        }

        if let Some((old_path, new_path)) =
            match_absolute_rename_entry(policy, rename_map, specifier)
        {
            return Some(SpecifierRenameMatch {
                old_path,
                new_path,
                package_name: package_name.map(ToString::to_string),
                package_directory: package_directory.map(Path::to_path_buf),
            });
        }

        return None;
    }

    // otherwise fall back to specifier shaped policy
    if let Some((old_path, new_path)) =
        match_relative_rename_entry(policy, source_path, rename_map, specifier)
    {
        return Some(SpecifierRenameMatch {
            old_path,
            new_path,
            package_name: None,
            package_directory: None,
        });
    }

    if let Some((old_path, new_path)) = match_file_uri_rename_entry(policy, rename_map, specifier) {
        return Some(SpecifierRenameMatch {
            old_path,
            new_path,
            package_name: None,
            package_directory: None,
        });
    }

    if let Some((old_path, new_path)) = match_absolute_rename_entry(policy, rename_map, specifier) {
        return Some(SpecifierRenameMatch {
            old_path,
            new_path,
            package_name: None,
            package_directory: None,
        });
    }

    if let Some((old_path, new_path, package_name, package_directory)) =
        match_package_rename_entry(rename_map, specifier)
    {
        return Some(SpecifierRenameMatch {
            old_path,
            new_path,
            package_name: Some(package_name),
            package_directory: Some(package_directory),
        });
    }

    let (old_path, new_path) = match_alias_rename_entry(rename_map, specifier)?;
    Some(SpecifierRenameMatch {
        old_path,
        new_path,
        package_name: None,
        package_directory: None,
    })
}

/// Apply one file rename to a module specifier.
pub(super) fn apply_rename_to_specifier(
    source_path: Option<&Path>,
    specifier: &str,
    rename_match: &SpecifierRenameMatch,
) -> Option<String> {
    // parse out query and fragment parts
    let parsed = ModuleSpecifier::parse(specifier);

    // preserve file specifiers by rewriting the file uri path
    if specifier.starts_with("file://") {
        let specifier_path = parsed.path.trim_start_matches("file://");
        if specifier_path.is_empty() {
            return None;
        }

        let updated_path = apply_common_suffix_rename(
            Path::new(specifier_path),
            &rename_match.old_path,
            &rename_match.new_path,
        );
        let mut updated = file_uri_for_path(&updated_path);
        if let Some(query) = parsed.query.as_ref() {
            updated.push_str(query);
        }
        if let Some(fragment) = parsed.fragment.as_ref() {
            updated.push_str(fragment);
        }

        return Some(updated);
    }

    // preserve extension style when present
    let specifier_path = parsed.path();
    let has_extension = Path::new(specifier_path).extension().is_some();
    let strip_extension = !has_extension;

    let updated = if specifier.starts_with("./") || specifier.starts_with("../") {
        let source_path = source_path?;
        build_relative_import_display_path(source_path, &rename_match.new_path, strip_extension)
    }
    // absolute specifiers
    else if specifier_path.starts_with('/') {
        let updated_path = apply_common_suffix_rename(
            Path::new(specifier_path),
            &rename_match.old_path,
            &rename_match.new_path,
        );
        let mut updated = normalize_separators(&updated_path.to_string_lossy());
        if strip_extension {
            updated = strip_module_extension(&updated);
        }
        updated
    }
    // alias specifiers
    else if is_alias_specifier(specifier_path) {
        let mut updated = apply_alias_rename(
            specifier_path,
            &rename_match.old_path,
            &rename_match.new_path,
        )?;
        if strip_extension {
            updated = strip_module_extension(&updated);
        }
        updated
    }
    // package specifiers
    else if let (Some(package_name), Some(package_directory)) = (
        rename_match.package_name.as_deref(),
        rename_match.package_directory.as_deref(),
    ) {
        let package_prefix = format!("{package_name}/");
        let specifier_suffix = specifier_path.strip_prefix(&package_prefix)?;
        if specifier_suffix.is_empty() {
            return None;
        }

        let relative = relative_path(package_directory, &rename_match.new_path)?;
        let mut relative_str = normalize_separators(&relative.to_string_lossy());
        if strip_extension {
            relative_str = strip_module_extension(&relative_str);
        }
        if relative_str.is_empty() || relative_str == ".." || relative_str.starts_with("../") {
            return None;
        }

        format!("{package_name}/{relative_str}")
    } else {
        return None;
    };

    // reattach query or fragment suffixes
    let mut updated = updated;
    if let Some(query) = parsed.query.as_ref() {
        updated.push_str(query);
    }
    if let Some(fragment) = parsed.fragment.as_ref() {
        updated.push_str(fragment);
    }

    Some(updated)
}

/// Resolve one rename entry for file uri specifiers.
fn match_file_uri_rename_entry(
    policy: &SpecifierPolicy<'_>,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    if !specifier.starts_with("file://") {
        return None;
    }

    let parsed = ModuleSpecifier::parse(specifier);
    let path_part = parsed.path.trim_start_matches("file://");
    if path_part.is_empty() {
        return None;
    }

    let path = Path::new(path_part).normalize();
    match_path_rename_entry(policy, rename_map, &path)
}

/// Resolve one rename entry for absolute path specifiers.
fn match_absolute_rename_entry(
    policy: &SpecifierPolicy<'_>,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    let parsed = ModuleSpecifier::parse(specifier);
    let path = Path::new(parsed.path());
    if !path.is_absolute() {
        return None;
    }

    let path = path.normalize();
    match_path_rename_entry(policy, rename_map, &path)
}

/// Resolve one rename entry for relative path specifiers.
fn match_relative_rename_entry(
    policy: &SpecifierPolicy<'_>,
    source_path: Option<&Path>,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None;
    }

    let source_path = source_path?;
    let source_directory = source_path.parent()?;
    let specifier_path = source_directory.join(specifier).normalize();

    match_path_rename_entry(policy, rename_map, &specifier_path)
}

/// Resolve one rename entry for alias specifiers when module resolution is unavailable.
fn match_alias_rename_entry(
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    let (_prefix, suffix) = split_alias_prefix(specifier)?;
    let suffix = suffix.trim_start_matches('/');
    if suffix.is_empty() {
        return None;
    }

    let suffix_path = Path::new(suffix);
    for (old_path, new_path) in rename_map {
        if strip_path_suffix(old_path, suffix_path).is_some() {
            return Some((old_path.clone(), new_path.clone()));
        }

        if suffix_path.extension().is_none() {
            let Some(old_no_extension) = strip_path_extension(old_path) else {
                continue;
            };
            if strip_path_suffix(&old_no_extension, suffix_path).is_some() {
                return Some((old_path.clone(), new_path.clone()));
            }
        }
    }

    None
}

/// Resolve one rename entry for package specifiers when module resolution is unavailable.
fn match_package_rename_entry(
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf, String, PathBuf)> {
    let (package_name, suffix) = split_package_specifier(specifier)?;
    let suffix = suffix.trim_start_matches('/');
    if suffix.is_empty() {
        return None;
    }

    let suffix_path = Path::new(suffix);
    for (old_path, new_path) in rename_map {
        let package_directory = package_root_for_path(old_path, &package_name)?;

        let Ok(relative) = old_path.strip_prefix(&package_directory) else {
            continue;
        };

        if relative == suffix_path {
            return Some((
                old_path.clone(),
                new_path.clone(),
                package_name.clone(),
                package_directory,
            ));
        }

        if suffix_path.extension().is_none() {
            let Some(relative_no_extension) = strip_path_extension(relative) else {
                continue;
            };
            if relative_no_extension == suffix_path {
                return Some((
                    old_path.clone(),
                    new_path.clone(),
                    package_name.clone(),
                    package_directory,
                ));
            }
        }
    }

    None
}

/// Split one package specifier into the package name and path suffix.
fn split_package_specifier(specifier: &str) -> Option<(String, String)> {
    let parsed = ModuleSpecifier::parse(specifier);
    let path = parsed.path();
    if path.is_empty() || path.starts_with('.') || path.starts_with('/') || is_alias_specifier(path)
    {
        return None;
    }

    let mut segments = path.split('/');
    let first = segments.next()?;
    if first.is_empty() {
        return None;
    }

    let package_name = if first.starts_with('@') {
        let second = segments.next()?;
        format!("{first}/{second}")
    } else {
        first.to_string()
    };

    let suffix = path.strip_prefix(&package_name)?;
    if !suffix.starts_with('/') {
        return None;
    }

    Some((package_name, suffix.to_string()))
}

/// Resolve the package root directory for one renamed path.
fn package_root_for_path(path: &Path, package_name: &str) -> Option<PathBuf> {
    let package_path = Path::new(package_name);
    let path_components = path.components().collect::<Vec<_>>();
    let package_components = package_path.components().collect::<Vec<_>>();

    for index in 0..path_components.len() {
        if path_components[index].as_os_str() != "node_modules" {
            continue;
        }

        let end = index + 1 + package_components.len();
        if end > path_components.len() {
            continue;
        }

        let package_matches = package_components
            .iter()
            .enumerate()
            .all(|(offset, component)| path_components[index + 1 + offset] == *component);
        if !package_matches {
            continue;
        }

        let package_directory =
            path_components[..end]
                .iter()
                .fold(PathBuf::new(), |mut current, component| {
                    current.push(component.as_os_str());
                    current
                });
        return Some(package_directory);
    }

    None
}

/// Resolve one rename entry for directory moves.
fn match_directory_rename_target(
    policy: &SpecifierPolicy<'_>,
    rename_map: &HashMap<PathBuf, PathBuf>,
    target_path: &Path,
) -> Option<(PathBuf, PathBuf)> {
    let absolute_target = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        policy.workspace_root.join(target_path)
    };

    for (old_path, new_path) in rename_map {
        if !is_directory_rename_entry(policy.fs, old_path, new_path) {
            continue;
        }

        let absolute_old = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            policy.workspace_root.join(old_path)
        };

        let Ok(relative) = absolute_target.strip_prefix(&absolute_old) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }

        let updated = if new_path.is_absolute() {
            new_path.join(relative)
        } else {
            policy.workspace_root.join(new_path).join(relative)
        };

        return Some((target_path.to_path_buf(), updated));
    }

    None
}

/// Match one specifier path to the best rename map entry.
fn match_path_rename_entry(
    policy: &SpecifierPolicy<'_>,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier_path: &Path,
) -> Option<(PathBuf, PathBuf)> {
    if let Some(new_path) = rename_map.get(specifier_path) {
        return Some((specifier_path.to_path_buf(), new_path.clone()));
    }

    for (old_path, new_path) in rename_map {
        let absolute_old = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            policy.workspace_root.join(old_path)
        };
        let absolute_new = if new_path.is_absolute() {
            new_path.to_path_buf()
        } else {
            policy.workspace_root.join(new_path)
        };

        if specifier_path == absolute_old {
            return Some((absolute_old, absolute_new));
        }

        if specifier_path.extension().is_none() {
            let Some(old_no_extension) = strip_path_extension(&absolute_old) else {
                continue;
            };
            if specifier_path == old_no_extension {
                return Some((absolute_old, absolute_new));
            }
        }

        if specifier_path.is_absolute() {
            let old_suffix = old_path
                .strip_prefix(policy.workspace_root)
                .unwrap_or(old_path.as_path());

            if strip_path_suffix(specifier_path, old_suffix).is_some() {
                return Some((old_path.clone(), new_path.clone()));
            }

            if specifier_path.extension().is_none() {
                let Some(old_no_extension) = strip_path_extension(old_suffix) else {
                    continue;
                };
                if strip_path_suffix(specifier_path, &old_no_extension).is_some() {
                    return Some((old_path.clone(), new_path.clone()));
                }
            }

            let shared_suffix = common_suffix_len(specifier_path, old_path);
            if shared_suffix >= 2 {
                return Some((old_path.clone(), new_path.clone()));
            }

            if specifier_path.extension().is_none() {
                let Some(old_no_extension) = strip_path_extension(old_path) else {
                    continue;
                };
                let shared_suffix_no_extension =
                    common_suffix_len(specifier_path, &old_no_extension);
                if shared_suffix_no_extension >= 2 {
                    return Some((old_path.clone(), new_path.clone()));
                }
            }
        }
    }

    for (old_path, new_path) in rename_map {
        if !is_directory_rename_entry(policy.fs, old_path, new_path) {
            continue;
        }

        let absolute_old = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            policy.workspace_root.join(old_path)
        };

        if let Ok(relative) = specifier_path.strip_prefix(&absolute_old)
            && !relative.as_os_str().is_empty()
        {
            let updated = if new_path.is_absolute() {
                new_path.join(relative)
            } else {
                policy.workspace_root.join(new_path).join(relative)
            };
            return Some((specifier_path.to_path_buf(), updated));
        }
    }

    None
}

/// Check whether one rename entry represents a directory move.
fn is_directory_rename_entry(fs: &dyn FileSystem, old_path: &Path, new_path: &Path) -> bool {
    if fs
        .metadata(old_path)
        .map(|metadata| metadata.is_directory)
        .unwrap_or(false)
    {
        return true;
    }

    if fs
        .metadata(new_path)
        .map(|metadata| metadata.is_directory)
        .unwrap_or(false)
    {
        return true;
    }

    old_path.extension().is_none() && new_path.extension().is_none()
}

/// Build one display path for a relative import rewrite.
fn build_relative_import_display_path(
    source_path: &Path,
    target_path: &Path,
    strip_extension: bool,
) -> String {
    let source_directory = source_path.parent().unwrap_or(source_path);
    let relative =
        relative_path(source_directory, target_path).unwrap_or_else(|| target_path.normalize());
    let mut display_path = normalize_separators(&relative.to_string_lossy());

    if !display_path.starts_with("./") && !display_path.starts_with("../") {
        display_path = format!("./{display_path}");
    }

    if strip_extension {
        display_path = strip_module_extension(&display_path);
    }

    display_path
}

/// Build one file uri from a path.
fn file_uri_for_path(path: &Path) -> String {
    let path_str = normalize_separators(&path.to_string_lossy());
    if path.is_absolute() && !path_str.starts_with('/') {
        return format!("file:///{path_str}");
    }

    format!("file://{path_str}")
}

/// Apply one file rename to an alias specifier.
fn apply_alias_rename(specifier: &str, old_path: &Path, new_path: &Path) -> Option<String> {
    let (prefix, suffix) = split_alias_prefix(specifier)?;
    let suffix = suffix.trim_start_matches('/');
    if suffix.is_empty() {
        return None;
    }

    let old_path = old_path.normalize();
    let new_path = new_path.normalize();
    let suffix_path = Path::new(suffix);

    let alias_root = strip_path_suffix(&old_path, suffix_path).or_else(|| {
        if suffix_path.extension().is_some() {
            return None;
        }

        let old_path_no_extension = strip_path_extension(&old_path)?;
        strip_path_suffix(&old_path_no_extension, suffix_path)
    })?;

    let relative = relative_path(&alias_root, &new_path)?;
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }

    let relative_str = normalize_separators(&relative.to_string_lossy());
    Some(format!("{prefix}{relative_str}"))
}

/// Check if one specifier is a known alias form.
fn is_alias_specifier(specifier: &str) -> bool {
    specifier.starts_with("@/") || specifier.starts_with("~/") || specifier.starts_with('#')
}

/// Split one specifier into its alias prefix and suffix.
fn split_alias_prefix(specifier: &str) -> Option<(&str, &str)> {
    if let Some(stripped) = specifier.strip_prefix("@/") {
        return Some(("@/", stripped));
    }
    if let Some(stripped) = specifier.strip_prefix("~/") {
        return Some(("~/", stripped));
    }
    if let Some(stripped) = specifier.strip_prefix("#/") {
        return Some(("#/", stripped));
    }
    if let Some(stripped) = specifier.strip_prefix('#') {
        return Some(("#", stripped));
    }

    None
}

/// Strip one code module extension from an import path.
fn strip_module_extension(path: &str) -> String {
    let extensions = [
        ".d.ts", ".d.mts", ".d.cts", ".d.ds", ".tsx", ".ts", ".mts", ".cts", ".jsx", ".js", ".mjs",
        ".cjs", ".ds",
    ];

    for extension in extensions {
        if let Some(stripped) = path.strip_suffix(extension) {
            return stripped.to_string();
        }
    }

    path.to_string()
}

/// Compute one relative path between two paths when possible.
fn relative_path(from_directory: &Path, to_path: &Path) -> Option<PathBuf> {
    let from_directory = from_directory.normalize();
    let to_path = to_path.normalize();

    let from_components = normal_components(&from_directory);
    let to_components = normal_components(&to_path);

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

    let mut relative = PathBuf::new();
    for _ in common..from_components.len() {
        relative.push("..");
    }

    for component in &to_components[common..] {
        relative.push(component);
    }

    Some(relative)
}

/// Normalize path separators to forward slashes.
fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Return the number of shared suffix components between two paths.
fn common_suffix_len(left: &Path, right: &Path) -> usize {
    let left_components: Vec<_> = left.components().collect();
    let right_components: Vec<_> = right.components().collect();
    let mut count = 0usize;

    while count < left_components.len() && count < right_components.len() {
        let left_index = left_components.len() - 1 - count;
        let right_index = right_components.len() - 1 - count;
        if left_components[left_index] != right_components[right_index] {
            break;
        }
        count += 1;
    }

    count
}

/// Apply one file rename by replacing the shared suffix with the new path suffix.
fn apply_common_suffix_rename(specifier_path: &Path, old_path: &Path, new_path: &Path) -> PathBuf {
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

/// Collect the last n components from a path.
fn path_suffix(path: &Path, component_count: usize) -> PathBuf {
    let components: Vec<_> = path.components().collect();
    let start = components.len().saturating_sub(component_count);
    let mut suffix = PathBuf::new();
    for component in &components[start..] {
        suffix.push(component.as_os_str());
    }

    suffix
}

/// Strip one path suffix, returning the prefix path.
fn strip_path_suffix(path: &Path, suffix: &Path) -> Option<PathBuf> {
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

/// Strip the extension from one path when it has one.
fn strip_path_extension(path: &Path) -> Option<PathBuf> {
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
    let mut components = Vec::new();

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

    components
}
