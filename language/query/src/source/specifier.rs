use std::path::{Path, PathBuf};

use tspp_artifact::{ExportTarget, PackageNode};
use tspp_repository::{
    ModulePathOutcome, ModulePathResolution, Repository, Revision, normalize_workspace_path,
};
use tspp_source::{FileId, ModuleId, PackageId, TSPP_FILE_TYPES};

use crate::source::{path_text, relative_path};
use crate::{ProgramQueryContext, QueryError, QueryResult};

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

/// Return the shortest relative specifier from one module path to another module.
pub(crate) fn relative_module_specifier(
    repository: &Repository,
    revision: Revision,
    source_path: &Path,
    target_module_id: ModuleId,
) -> QueryResult<Option<String>> {
    let Some(target_path) = canonical_module_path(repository, revision, target_module_id)? else {
        return Ok(None);
    };
    let source_directory = source_path.parent().ok_or(QueryError::invalid(format!(
        "module import base: {source_path:?}"
    )))?;
    let relative = relative_path(source_directory, &target_path).ok_or(QueryError::invalid(
        format!("relative module path: {source_path:?} -> {target_module_id:?}"),
    ))?;
    let mut specifier = path_text(&relative)?;
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        specifier = format!("./{specifier}");
    }

    Ok(Some(specifier))
}

impl ProgramQueryContext<'_> {
    /// Return every active package export and its selected program module.
    pub(crate) fn package_module_exports(
        &self,
        package_id: PackageId,
    ) -> QueryResult<Vec<(String, ModuleId)>> {
        let repository = self.repository();
        let revision = self.revision();
        let package = self.package_node(package_id)?;

        // resolve implicit builtin exports through their canonical specifiers
        if repository.is_builtin_package(package_id) {
            let mut exports = Vec::new();
            for (key, target) in &package.exports.exact {
                if !target.is_module {
                    continue;
                }
                let specifier = builtin_specifier(key)?;
                let uri = repository
                    .builtin_module_uri_for_specifier(revision, &specifier, &mut Vec::new())?
                    .ok_or_else(|| {
                        QueryError::invalid(format!(
                            "builtin package export {key:?} has no module URI"
                        ))
                    })?;
                let selected = repository
                    .module_id_for_uri(revision, &uri)?
                    .ok_or_else(|| QueryError::missing(format!("builtin export module: {uri}")))?;
                exports.push((key.clone(), selected));
            }

            return Ok(exports);
        }

        let Some(package_root) = package.root.as_deref() else {
            return Err(QueryError::missing(format!("package root: {package_id:?}")));
        };
        let mut exports = Vec::new();

        // resolve every exact export once
        for (key, target) in &package.exports.exact {
            let path = package_root.join(&target.path);
            if let Some(module_id) =
                resolve_export_module(repository, revision, package_id, target, &path)?
            {
                exports.push((key.clone(), module_id));
            }
        }

        // invert pattern exports through each package module once
        for module_id in self.module_ids() {
            if module_id.package_id != package_id {
                continue;
            }
            let module = repository
                .module(revision, *module_id)?
                .ok_or_else(|| QueryError::missing(format!("repository module: {module_id:?}")))?;
            let Some(module_path) = module.path.as_deref() else {
                continue;
            };
            for key in pattern_export_keys(
                repository,
                revision,
                package_id,
                &package,
                package_root,
                *module_id,
                module_path,
            )? {
                exports.push((key, *module_id));
            }
        }
        exports.sort();
        exports.dedup();

        Ok(exports)
    }
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

/// Build one builtin package specifier from a public export key.
pub(crate) fn builtin_specifier(export_key: &str) -> QueryResult<String> {
    // package root
    if export_key == "." {
        Ok("tspp:".to_string())
    }
    // package subpath
    else if let Some(export_path) = export_key.strip_prefix("./")
        && !export_path.is_empty()
    {
        Ok(format!("tspp:{export_path}"))
    }
    // invalid public key
    else {
        Err(QueryError::invalid(format!(
            "invalid builtin package export key: {export_key:?}"
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

/// Return pattern export keys selecting one exact module.
fn pattern_export_keys(
    repository: &Repository,
    revision: Revision,
    package_id: PackageId,
    package: &PackageNode,
    package_root: &Path,
    module_id: ModuleId,
    module_path: &Path,
) -> QueryResult<Vec<String>> {
    let mut keys = Vec::new();

    // invert each pattern through the module's addressable paths
    for pattern in &package.exports.patterns {
        let target_pattern = package_root.join(&pattern.target.path);
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
            if resolved == Some(module_id) {
                keys.push(key);
            }
        }
    }

    Ok(keys)
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
    let is_source = TSPP_FILE_TYPES
        .iter()
        .filter_map(|file_type| file_type.extension())
        .any(|candidate| candidate == extension);
    if !is_source {
        return path.to_path_buf();
    }

    let mut path = path.to_path_buf();
    path.set_extension("");

    path
}
