use std::path::{Path, PathBuf};

use destack_artifact::{ExportTarget, PackageNode};
use destack_repository::{
    ModulePathOutcome, ModulePathResolution, Repository, Revision, normalize_workspace_path,
};
use destack_source::{CODE_FILE_TYPES, FileId, FileType, ModuleId, PackageId};

use crate::source::path_text;
use crate::{QueryError, QueryResult};

/// Return the shortest exact import target path for one module.
pub(crate) fn canonical_module_path(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> QueryResult<Option<PathBuf>> {
    let module = repository
        .module(revision, module_id)?
        .ok_or_else(|| QueryError::missing(format!("repository module: {module_id:?}")))?;
    let Some(explicit_path) = module.path.as_deref() else {
        return Ok(None);
    };
    if !resolution_selects_module(
        repository,
        revision,
        explicit_path,
        module_id,
        module.file_id,
    )? {
        return Ok(None);
    }

    // prefer the extensionless form only when it selects the same primary module
    let extensionless_path = strip_module_extension_path(explicit_path);
    if extensionless_path != explicit_path
        && resolution_selects_module(
            repository,
            revision,
            &extensionless_path,
            module_id,
            module.file_id,
        )?
    {
        Ok(Some(extensionless_path))
    } else {
        Ok(Some(explicit_path.to_path_buf()))
    }
}

/// Return public export keys of one dependency package selecting one module.
pub(crate) fn export_keys_selecting_module(
    repository: &Repository,
    revision: Revision,
    package_id: PackageId,
    package: &PackageNode,
    module_id: ModuleId,
) -> QueryResult<Vec<String>> {
    let Some(package_root) = package.root.as_deref() else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();

    // resolve each exact export once
    for (key, target) in &package.exports.exact {
        let path = package_root.join(&target.path);
        let resolved = resolve_export_module(repository, revision, package_id, target, &path)?;
        if resolved == Some(module_id) {
            keys.push(key.clone());
        }
    }

    // invert pattern exports through the module's addressable paths
    let module = repository
        .module(revision, module_id)?
        .ok_or_else(|| QueryError::missing(format!("repository module: {module_id:?}")))?;
    if let Some(module_path) = module.path.as_deref() {
        for pattern in &package.exports.patterns {
            let target_pattern = package_root.join(&pattern.target.path);

            // exclude targets rejected by normal forward import resolution
            let Some(target_pattern) = normalize_workspace_path(target_pattern) else {
                continue;
            };
            let target_pattern = path_text(&target_pattern)?;

            for module_path in module_match_paths(module_path)? {
                let Some(replacement) = wildcard_replacement(&target_pattern, &module_path) else {
                    continue;
                };
                let key = format!("{}{}{}", pattern.prefix, replacement, pattern.suffix);
                let Some(export) = package.exports.get(&key) else {
                    continue;
                };
                let path = package_root.join(export.path.as_ref());
                let resolved =
                    resolve_export_module(repository, revision, package_id, export.target, &path)?;
                if resolved == Some(module_id) && !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    }

    Ok(keys)
}

/// Build one external package specifier from a package name and export key.
pub(crate) fn package_specifier(package_name: &str, export_key: &str) -> QueryResult<String> {
    // package root
    if export_key == "." {
        Ok(package_name.to_string())
    }
    // package subpath
    else if let Some(export_path) = export_key.strip_prefix("./")
        && !export_path.is_empty()
    {
        Ok(format!("{package_name}/{export_path}"))
    }
    // invalid public key
    else {
        Err(QueryError::invalid(format!(
            "invalid public package export key: {export_key:?}"
        )))
    }
}

/// Return whether one logical path selects one exact primary module.
fn resolution_selects_module(
    repository: &Repository,
    revision: Revision,
    path: &Path,
    expected_module: ModuleId,
    expected_file: FileId,
) -> QueryResult<bool> {
    match probe_module_path(repository, revision, path)?.outcome {
        ModulePathOutcome::Resolved { path, module } => {
            let is_expected =
                module == expected_module && repository.file_id(&path) == expected_file;

            Ok(is_expected)
        }
        ModulePathOutcome::Unsupported
        | ModulePathOutcome::Missing
        | ModulePathOutcome::Ambiguous { .. } => Ok(false),
    }
}

/// Resolve one public export target to one exact primary module.
fn resolve_export_module(
    repository: &Repository,
    revision: Revision,
    package_id: PackageId,
    target: &ExportTarget,
    path: &Path,
) -> QueryResult<Option<ModuleId>> {
    if !target.is_module {
        return Ok(None);
    }

    let ModulePathOutcome::Resolved { path, module } =
        probe_module_path(repository, revision, path)?.outcome
    else {
        return Ok(None);
    };

    // retain exact primary modules from the exported package
    if module.package_id == package_id {
        let resolved = repository
            .module(revision, module)?
            .ok_or_else(|| QueryError::missing(format!("repository module: {module:?}")))?;
        if repository.file_id(&path) == resolved.file_id {
            return Ok(Some(module));
        }
    }

    Ok(None)
}

/// Resolve one path without retaining the query-side probes.
fn probe_module_path(
    repository: &Repository,
    revision: Revision,
    path: &Path,
) -> QueryResult<ModulePathResolution> {
    Ok(repository.resolve_module_path(revision, path, None)?)
}

/// Return normalized module path forms addressable by package exports.
fn module_match_paths(module_path: &Path) -> QueryResult<Vec<String>> {
    let path = path_text(module_path)?;
    let mut paths = vec![path];

    // include the extensionless source path
    let extensionless = strip_module_extension_path(module_path);
    if extensionless != module_path {
        paths.push(path_text(&extensionless)?);
    }

    Ok(paths)
}

/// Return the wildcard text that reproduces one concrete path.
fn wildcard_replacement(pattern: &str, path: &str) -> Option<String> {
    let (prefix, _) = pattern.split_once('*')?;
    if !path.starts_with(prefix) {
        return None;
    }

    // try each character boundary because one replacement may fill multiple wildcards
    for (end, _) in path
        .char_indices()
        .chain(std::iter::once((path.len(), '\0')))
    {
        if end < prefix.len() {
            continue;
        }

        let replacement = &path[prefix.len()..end];
        if pattern.replace('*', replacement) == path {
            return Some(replacement.to_string());
        }
    }

    None
}

/// Remove one recognized source extension from a module path.
fn strip_module_extension_path(path: &Path) -> PathBuf {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return path.to_path_buf();
    };
    let is_source = CODE_FILE_TYPES
        .iter()
        .filter_map(FileType::extension)
        .any(|candidate| candidate == extension);
    if !is_source {
        return path.to_path_buf();
    }

    let mut path = path.to_path_buf();
    path.set_extension("");

    path
}
