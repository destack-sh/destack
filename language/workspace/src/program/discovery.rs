use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{ModuleId, PackageId, matches as glob_matches};

use crate::{ModuleRegistry, Target, TargetDiscovery, TargetId};

/// Select the source of entry points for entry discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySource {
    /// Use only target entry paths.
    Target,
    /// Use only package manifest entry targets.
    Manifest,
    /// Use target entries first, then manifest entry targets.
    Auto,
}

/// Select how entry paths are resolved against module registry paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryResolutionMode {
    /// Resolve entries relative to package path only.
    Strict,
    /// Resolve entries relative to package path, then as registry paths.
    RegistryRelative,
}

/// Options for entry discovery behavior.
#[derive(Debug, Clone)]
pub struct TargetDiscoveryOptions<'a> {
    /// Source policy for entry paths.
    pub entry_source: EntrySource,
    /// Resolution policy for selected entry paths.
    pub entry_resolution: EntryResolutionMode,
    /// Manifest entry targets from package.json fields.
    pub manifest_entry_targets: &'a [String],
}

impl<'a> Default for TargetDiscoveryOptions<'a> {
    fn default() -> Self {
        Self {
            entry_source: EntrySource::Target,
            entry_resolution: EntryResolutionMode::Strict,
            manifest_entry_targets: &[],
        }
    }
}

/// Describe a failure while discovering target modules.
#[derive(Debug, Clone)]
pub enum TargetDiscoveryIssue {
    /// Missing package path for entry based discovery.
    MissingPackagePath {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
    },
    /// Missing entry module for entry based discovery.
    MissingEntry {
        /// Package id for the discovery.
        package: PackageId,
        /// Target id for the discovery.
        target: TargetId,
        /// Entry path that could not be resolved.
        path: PathBuf,
    },
}

/// Discover modules for one target using the selected discovery strategy.
pub fn discover_target_modules(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
    target_id: &TargetId,
    options: &TargetDiscoveryOptions<'_>,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    // dispatch discovery based on target mode
    match target.discovery {
        TargetDiscovery::Entry => discover_entry_modules(
            modules,
            package_id,
            package_path,
            target,
            target_id,
            options,
        ),
        TargetDiscovery::Include => {
            discover_include_modules(modules, package_id, package_path, target)
        }
    }
}

/// Discover modules from target entry points.
pub fn discover_entry_modules(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
    target_id: &TargetId,
    options: &TargetDiscoveryOptions<'_>,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    // choose entry paths based on the configured source policy
    let entry_paths = select_entry_paths(
        modules,
        package_id,
        package_path,
        target,
        target_id,
        options,
    )?;

    resolve_entry_modules(
        modules,
        package_id,
        package_path,
        target_id,
        &entry_paths,
        options.entry_resolution,
    )
}

/// Select effective entry paths for one target.
fn select_entry_paths(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
    target_id: &TargetId,
    options: &TargetDiscoveryOptions<'_>,
) -> Result<Vec<PathBuf>, TargetDiscoveryIssue> {
    // use target entry configuration directly
    if matches!(options.entry_source, EntrySource::Target) {
        return Ok(target.entry.clone());
    }

    // use target entries first when auto source has explicit entries
    if matches!(options.entry_source, EntrySource::Auto) && !target.entry.is_empty() {
        return Ok(target.entry.clone());
    }

    // manifest based entry selection requires package path context
    let package_dir =
        package_path
            .as_ref()
            .ok_or_else(|| TargetDiscoveryIssue::MissingPackagePath {
                package: package_id,
                target: target_id.clone(),
            })?;

    // skip when no manifest entry targets were provided
    if options.manifest_entry_targets.is_empty() {
        return Ok(Vec::new());
    }

    // collect candidate module paths that belong to the package
    let mut candidates = Vec::new();
    for module_ref in modules.iter() {
        let module = module_ref.read();
        if module.package_id != package_id {
            continue;
        }
        let Some(path) = module.path.clone() else {
            continue;
        };
        candidates.push(path);
    }

    Ok(select_manifest_entry_paths(
        package_dir,
        &candidates,
        options.manifest_entry_targets,
    ))
}

/// Resolve selected entry paths to package-local module ids.
fn resolve_entry_modules(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target_id: &TargetId,
    entry_paths: &[PathBuf],
    mode: EntryResolutionMode,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    let mut discovered_modules = Vec::new();

    // resolve each entry path with the configured mode
    for entry_path in entry_paths {
        let module_id = match mode {
            EntryResolutionMode::Strict => resolve_entry_module_strict(
                modules,
                package_id,
                package_path,
                target_id,
                entry_path,
            )?,
            EntryResolutionMode::RegistryRelative => resolve_entry_module_registry_relative(
                modules,
                package_id,
                package_path,
                target_id,
                entry_path,
            )?,
        };

        discovered_modules.push(module_id);
    }

    Ok(discovered_modules)
}

/// Resolve one entry path relative to package path only.
fn resolve_entry_module_strict(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target_id: &TargetId,
    entry_path: &Path,
) -> Result<ModuleId, TargetDiscoveryIssue> {
    // require a package path for strict entry resolution
    let package_dir =
        package_path
            .as_ref()
            .ok_or_else(|| TargetDiscoveryIssue::MissingPackagePath {
                package: package_id,
                target: target_id.clone(),
            })?;

    // build the path relative to package root when needed
    let resolved_path = if entry_path.is_absolute() {
        entry_path.to_path_buf()
    } else {
        package_dir.join(entry_path)
    };

    resolve_entry_module_id(modules, package_id, &resolved_path).ok_or_else(|| {
        TargetDiscoveryIssue::MissingEntry {
            package: package_id,
            target: target_id.clone(),
            path: resolved_path,
        }
    })
}

/// Resolve one entry path with package-relative and registry-relative checks.
fn resolve_entry_module_registry_relative(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target_id: &TargetId,
    entry_path: &Path,
) -> Result<ModuleId, TargetDiscoveryIssue> {
    let mut candidate_paths = Vec::new();

    // prefer package relative path first
    if !entry_path.is_absolute() {
        let package_dir =
            package_path
                .as_ref()
                .ok_or_else(|| TargetDiscoveryIssue::MissingPackagePath {
                    package: package_id,
                    target: target_id.clone(),
                })?;
        candidate_paths.push(package_dir.join(entry_path));
    }

    // fall back to direct registry path
    candidate_paths.push(entry_path.to_path_buf());

    // return the first package local module id that resolves
    for candidate_path in &candidate_paths {
        if let Some(module_id) = resolve_entry_module_id(modules, package_id, candidate_path) {
            return Ok(module_id);
        }
    }

    let missing_path = candidate_paths
        .first()
        .cloned()
        .unwrap_or_else(|| entry_path.to_path_buf());
    Err(TargetDiscoveryIssue::MissingEntry {
        package: package_id,
        target: target_id.clone(),
        path: missing_path,
    })
}

/// Resolve one entry path to a package-local module id.
fn resolve_entry_module_id(
    modules: &ModuleRegistry,
    package_id: PackageId,
    entry_path: &std::path::Path,
) -> Option<ModuleId> {
    let module_id = modules.get_id_by_path(entry_path)?;
    let module_ref = modules.get(module_id);
    let module = module_ref.read();
    if module.package_id != package_id {
        return None;
    }

    Some(module_id)
}

/// Discover modules matching include and exclude patterns.
pub fn discover_include_modules(
    modules: &ModuleRegistry,
    package_id: PackageId,
    package_path: &Option<PathBuf>,
    target: &Target,
) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
    let mut discovered_modules = Vec::new();

    // scan modules that belong to the package
    for module_ref in modules.iter() {
        let module = module_ref.read();
        if module.package_id != package_id {
            continue;
        }

        // compute module path relative to the package
        let relative_path = match (&module.path, package_path) {
            (Some(module_path), Some(package_path)) => module_path
                .strip_prefix(package_path)
                .ok()
                .map(|path| path.to_path_buf()),
            _ => None,
        };

        // include pathless modules only when include patterns are empty
        let Some(relative_path) = relative_path else {
            if target.include.is_empty() {
                discovered_modules.push(module.id);
            }

            continue;
        };

        // skip modules excluded by pattern
        let path_text = relative_path.to_string_lossy();
        let path_bytes = path_text.as_bytes();
        let is_excluded = target
            .exclude
            .iter()
            .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
        if is_excluded {
            continue;
        }

        // keep modules included by pattern, or all when include is empty
        let is_included = target.include.is_empty()
            || target
                .include
                .iter()
                .any(|pattern| glob_matches(pattern.as_bytes(), 0, path_bytes, 0));
        if is_included {
            discovered_modules.push(module.id);
        }
    }

    Ok(discovered_modules)
}

/// Select manifest entry candidates from package.json target strings.
pub fn select_manifest_entry_paths(
    package_dir: &Path,
    candidates: &[PathBuf],
    entry_targets: &[String],
) -> Vec<PathBuf> {
    // skip empty inputs early
    if candidates.is_empty() || entry_targets.is_empty() {
        return Vec::new();
    }

    // build candidate keys once for target matching
    let candidate_infos = build_candidate_infos(package_dir, candidates);
    if candidate_infos.is_empty() {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut selected_keys = HashSet::new();

    // collect matching candidates while preserving target order
    for target in entry_targets {
        let Some(candidate) =
            resolve_entry_target_to_candidate(package_dir, target, &candidate_infos)
        else {
            continue;
        };
        if !selected_keys.insert(candidate.key.clone()) {
            continue;
        }
        selected.push(candidate.path.clone());
    }

    selected
}

/// Candidate metadata used while resolving package entry targets.
#[derive(Debug, Clone)]
struct TargetCandidate {
    /// Absolute candidate path.
    path: PathBuf,
    /// Normalized key relative to package root.
    key: String,
    /// Key with supported module suffix removed.
    stem_key: String,
}

/// Build normalized candidate metadata.
fn build_candidate_infos(package_dir: &Path, candidates: &[PathBuf]) -> Vec<TargetCandidate> {
    candidates
        .iter()
        .map(|path| {
            let key = normalized_relative_key(package_dir, path);
            let stem_key = strip_supported_module_suffix(key.as_str()).to_string();
            TargetCandidate {
                path: path.clone(),
                key,
                stem_key,
            }
        })
        .collect()
}

/// Resolve one manifest target to one discovered candidate.
fn resolve_entry_target_to_candidate<'a>(
    package_dir: &Path,
    target: &str,
    candidates: &'a [TargetCandidate],
) -> Option<&'a TargetCandidate> {
    let normalized_target = normalize_manifest_target_path(package_dir, target)?;
    let target_key = normalized_relative_key(package_dir, &normalized_target);

    // prefer exact key matches first
    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.key == target_key)
    {
        return Some(candidate);
    }

    // support extension substitution and directory style index targets
    let mut target_stems = Vec::new();
    let target_stem = strip_supported_module_suffix(&target_key).to_string();
    target_stems.push(target_stem.clone());
    let trimmed = target_stem.trim_end_matches('/');
    if !trimmed.is_empty() {
        target_stems.push(format!("{trimmed}/index"));
    }

    candidates
        .iter()
        .find(|candidate| target_stems.iter().any(|stem| candidate.stem_key == *stem))
}

/// Normalize one package entry target into an absolute path candidate.
fn normalize_manifest_target_path(package_dir: &Path, target: &str) -> Option<PathBuf> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    // strip query and fragment suffixes
    let target = target
        .split_once('?')
        .map(|(value, _)| value)
        .unwrap_or(target);
    let target = target
        .split_once('#')
        .map(|(value, _)| value)
        .unwrap_or(target);
    if target.is_empty() {
        return None;
    }

    // skip non path targets
    if target.starts_with("node:")
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("file://")
        || target.contains('*')
        || target.starts_with('#')
    {
        return None;
    }

    // skip obvious bare package specifiers
    if !target.starts_with('.')
        && !target.starts_with('/')
        && !target.contains('/')
        && !target.contains('\\')
        && !target.contains('.')
    {
        return None;
    }

    let relative = target.strip_prefix("./").unwrap_or(target);
    let relative = relative.strip_prefix('/').unwrap_or(relative);
    if relative.is_empty() {
        return None;
    }

    Some(package_dir.join(relative))
}

/// Build a normalized relative key for path matching.
fn normalized_relative_key(package_dir: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(package_dir).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

/// Strip common source module suffixes from one key.
fn strip_supported_module_suffix(value: &str) -> &str {
    for suffix in [
        ".d.ts", ".d.mts", ".d.cts", ".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs",
    ] {
        if let Some(stripped) = value.strip_suffix(suffix) {
            return stripped;
        }
    }

    value
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::select_manifest_entry_paths;

    /// Match ts entry files from manifest targets with javascript suffixes.
    #[test]
    fn test_select_manifest_entry_paths_typescript_suffix_substitution() {
        let package_dir = PathBuf::from("/tmp/pkg");
        let candidates = vec![
            package_dir.join("src/index.ts"),
            package_dir.join("src/worker.ts"),
        ];
        let entry_targets = vec!["./src/index.js".to_string(), "./src/worker.js".to_string()];

        let selected = select_manifest_entry_paths(&package_dir, &candidates, &entry_targets);

        assert_eq!(selected, candidates);
    }

    /// Skip duplicate and missing manifest target matches.
    #[test]
    fn test_select_manifest_entry_paths_skip_duplicates_and_missing() {
        let package_dir = PathBuf::from("/tmp/pkg");
        let index = package_dir.join("src/index.ts");
        let candidates = vec![index.clone()];
        let entry_targets = vec![
            "./src/index.js".to_string(),
            "./src/index.ts".to_string(),
            "./src/missing.js".to_string(),
        ];

        let selected = select_manifest_entry_paths(&package_dir, &candidates, &entry_targets);

        assert_eq!(selected, vec![index]);
    }
}
