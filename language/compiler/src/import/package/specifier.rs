use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};

use destack_artifact::{ExportTarget, PackageDependency, PackageNode};
use destack_repository::Revision;
use destack_source::{CODE_FILE_TYPES, FileId, FileType, ModuleId, PackageId};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

use crate::import::ModulePathResolution;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Index canonical same-package import target paths over one program module set.
    pub(in crate::import) fn index_module_paths(
        &self,
        revision: Revision,
        modules: &[ModuleId],
    ) -> CompilerResult<Vec<(ModuleId, PathBuf)>> {
        let mut module_paths = Vec::new();

        // select the shortest exact import target path for each module
        for module_id in modules {
            let Some(path) = self.canonical_module_path(revision, *module_id)? else {
                continue;
            };

            module_paths.push((*module_id, path));
        }

        Ok(module_paths)
    }

    /// Return the shortest exact import target path for one module.
    fn canonical_module_path(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> CompilerResult<Option<PathBuf>> {
        let module = self.module(revision, module_id)?;
        let Some(explicit_path) = module.path.as_deref() else {
            return Ok(None);
        };
        if !self.resolution_selects_module(revision, explicit_path, module_id, module.file_id)? {
            return Ok(None);
        }

        // prefer the extensionless form only when it selects the same primary module
        let extensionless_path = strip_source_extension(explicit_path);
        if extensionless_path != explicit_path
            && self.resolution_selects_module(
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

    /// Return whether one logical path selects one exact primary module.
    fn resolution_selects_module(
        &self,
        revision: Revision,
        path: &Path,
        expected_module: ModuleId,
        expected_file: FileId,
    ) -> CompilerResult<bool> {
        let resolution = self.resolve_module_path(revision, path, None)?;
        match resolution {
            ModulePathResolution::Resolved { path, module, .. } => {
                let is_expected =
                    module == expected_module && self.repository.file_id(&path) == expected_file;

                Ok(is_expected)
            }
            ModulePathResolution::Unsupported
            | ModulePathResolution::Missing { .. }
            | ModulePathResolution::Ambiguous { .. } => Ok(false),
        }
    }

    /// Build public package specifiers for active dependency aliases.
    pub(in crate::import) fn index_package_specifiers(
        &self,
        revision: Revision,
        packages: &IndexMap<PackageId, PackageNode>,
        modules: &[ModuleId],
    ) -> CompilerResult<Vec<(PackageId, ModuleId, String)>> {
        let mut entries = Vec::new();
        let mut exports_by_package = FxHashMap::default();
        let mut modules_by_package = FxHashMap::<PackageId, Vec<ModuleId>>::default();

        // group modules once for pattern export inversion
        for module in modules {
            modules_by_package
                .entry(module.package_id)
                .or_default()
                .push(*module);
        }

        // index every active dependency alias from each source package
        for (source_package_id, source_package) in packages {
            let source_package_id = *source_package_id;

            for (package_name, dependency) in &source_package.dependencies {
                let PackageDependency::Resolved(target_package_id) = dependency else {
                    continue;
                };
                let target_package =
                    packages
                        .get(target_package_id)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "dependency package is absent from PackageGraph: \
                             {target_package_id:?}"
                            ),
                        })?;
                let exports = match exports_by_package.entry(*target_package_id) {
                    // reuse exports shared by dependency aliases
                    Entry::Occupied(entry) => entry.into_mut(),

                    // resolve one dependency package's exports once
                    Entry::Vacant(entry) => {
                        let package_modules = match modules_by_package.get(target_package_id) {
                            Some(modules) => modules.as_slice(),
                            None => &[],
                        };
                        let exports = self.package_exports(
                            revision,
                            *target_package_id,
                            target_package,
                            package_modules,
                        )?;

                        entry.insert(exports)
                    }
                };

                // combine the dependency alias with each exact public export
                for (module_id, export_key) in exports {
                    let specifier = package_specifier(package_name, export_key)?;
                    entries.push((source_package_id, *module_id, specifier));
                }
            }
        }

        Ok(entries)
    }

    /// Return exact public export keys from one dependency package.
    fn package_exports(
        &self,
        revision: Revision,
        package_id: PackageId,
        package: &PackageNode,
        modules: &[ModuleId],
    ) -> CompilerResult<Vec<(ModuleId, String)>> {
        let mut exports = Vec::new();
        let has_module_exports = package
            .exports
            .exact
            .values()
            .any(|target| target.is_module)
            || package
                .exports
                .patterns
                .iter()
                .any(|pattern| pattern.target.is_module);
        if !has_module_exports {
            return Ok(exports);
        }
        let package_root = package
            .root
            .as_deref()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "dependency package has module exports but no root path: {package_id:?}"
                ),
            })?;

        // resolve each exact export once
        for (key, target) in &package.exports.exact {
            let path = package_root.join(&target.path);
            let Some(module_id) =
                self.resolve_export_module(revision, package_id, target, &path)?
            else {
                continue;
            };
            exports.push((module_id, key.clone()));
        }

        // invert pattern exports through package modules
        for module_id in modules {
            let export_keys =
                self.pattern_export_keys(revision, package_id, package, package_root, *module_id)?;
            for export_key in export_keys {
                exports.push((*module_id, export_key));
            }
        }

        Ok(exports)
    }

    /// Return pattern export keys that select one exact module.
    fn pattern_export_keys(
        &self,
        revision: Revision,
        package_id: PackageId,
        package: &PackageNode,
        package_root: &Path,
        module_id: ModuleId,
    ) -> CompilerResult<Vec<String>> {
        let module = self.module(revision, module_id)?;
        let Some(module_path) = module.path.as_deref() else {
            return Ok(Vec::new());
        };
        let mut keys = Vec::new();

        // invert each pattern, then apply normal export precedence and resolution
        for pattern in &package.exports.patterns {
            let target_pattern = package_root.join(&pattern.target.path);

            // exclude targets rejected by normal forward import resolution
            let Some(target_pattern) = self.normalize_workspace_path(target_pattern) else {
                continue;
            };
            let target_pattern = path_text(&target_pattern)?;

            for module_path in module_match_paths(module_path)? {
                let Some(replacement) = wildcard_replacement(&target_pattern, &module_path) else {
                    continue;
                };
                let key = format!("{}{}{}", pattern.prefix, replacement, pattern.suffix);
                let export = package
                    .exports
                    .get(&key)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!(
                            "generated package export key is absent from its index: {key:?}"
                        ),
                    })?;
                let path = package_root.join(export.path.as_ref());
                let resolved =
                    self.resolve_export_module(revision, package_id, export.target, &path)?;
                if resolved == Some(module_id) && !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }

        Ok(keys)
    }

    /// Resolve one public export target to one exact primary module.
    fn resolve_export_module(
        &self,
        revision: Revision,
        package_id: PackageId,
        target: &ExportTarget,
        path: &Path,
    ) -> CompilerResult<Option<ModuleId>> {
        if !target.is_module {
            return Ok(None);
        }

        let resolution = self.resolve_module_path(revision, path, None)?;
        let ModulePathResolution::Resolved { path, module, .. } = resolution else {
            return Ok(None);
        };

        // retain exact primary modules from the exported package
        if module.package_id == package_id {
            let resolved = self.module(revision, module)?;
            if self.repository.file_id(&path) == resolved.file_id {
                return Ok(Some(module));
            }
        }

        Ok(None)
    }
}

/// Build one external package specifier from a package name and export key.
fn package_specifier(package_name: &str, export_key: &str) -> CompilerResult<String> {
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
        Err(CompilerError::Internal {
            message: format!("invalid public package export key: {export_key:?}"),
        })
    }
}

/// Return normalized module path forms addressable by package exports.
fn module_match_paths(module_path: &Path) -> CompilerResult<Vec<String>> {
    let path = path_text(module_path)?;
    let mut paths = vec![path];

    // include the extensionless source path
    let extensionless = strip_source_extension(module_path);
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
fn strip_source_extension(path: &Path) -> PathBuf {
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

/// Return one logical path using module specifier separators.
fn path_text(path: &Path) -> CompilerResult<String> {
    let path = path.to_str().ok_or_else(|| CompilerError::Internal {
        message: format!("module path is not valid Unicode: {path:?}"),
    })?;

    Ok(path.replace('\\', "/"))
}
