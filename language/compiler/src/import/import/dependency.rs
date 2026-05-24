use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use destack_dir as dir;
use destack_source::{
    CODE_FILE_TYPES, FileType, Loader, ModuleId, ModuleSpecifier, PackageId, Uri,
};
use destack_workspace::{ConditionSet, Dependency, ExportKind, Package, PackageExport};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult, DiagnosticAnchor, ImportError};

use super::specifier::{DependencySpecifier, PackageSpecifier};

impl Compiler {
    /// Import one dependency edge.
    pub(in crate::import) fn collect_dependency(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
        attributes: Option<&dir::ImportAttributeClause>,
        relation: dir::DependencyRelation,
    ) -> CompilerResult<()> {
        let anchor = state.anchor_node(expression_id.id)?;
        let specifier_text = state.strings().get(specifier).to_string();

        // read loader override
        let loader = state.extract_module_loader(&anchor, attributes);

        // resolve target module
        let target = self.resolve_dependency_module(state, &anchor, &specifier_text, loader)?;

        // append dependency edge
        let dependency = dir::DependencyEdge {
            source: expression_id.into_global_any(state.module.id),
            specifier,
            relation,
            loader,
            target,
        };

        state.push_dependency(dependency);

        Ok(())
    }

    /// Resolve the target module for one dependency specifier.
    fn resolve_dependency_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let specifier_parts = DependencySpecifier::parse(specifier);

        // resolve builtin module edges in builtin package space
        if self
            .repository
            .builtin_package()
            .contains_uri(state.module.uri.as_ref())
        {
            self.resolve_builtin_dependency_module(state, anchor, specifier, specifier_parts)
        }
        // resolve user module edges through package space
        else {
            self.resolve_user_dependency_module(state, anchor, specifier, specifier_parts, loader)
        }
    }

    /// Resolve one builtin package dependency module.
    fn resolve_builtin_dependency_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        specifier_parts: DependencySpecifier,
    ) -> CompilerResult<Option<ModuleId>> {
        let builtin = self.repository.builtin_package();

        // resolve absolute builtin specifier
        if let Some(uri) = builtin.module_uri_for_specifier(specifier) {
            return self.resolve_dependency_uri(state, anchor, specifier, &uri);
        }

        match specifier_parts {
            // resolve relative builtin specifier
            DependencySpecifier::Relative(specifier_parts) => {
                let specifier = specifier_parts.path();
                let uri =
                    builtin.module_uri_for_relative_specifier(state.module.uri.as_ref(), specifier);

                // relative builtin module exists
                if let Some(uri) = uri {
                    self.resolve_dependency_uri(state, anchor, specifier, &uri)
                }
                // relative builtin module escapes the package
                else {
                    state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                        anchor: anchor.clone(),
                        target: specifier.to_string(),
                    });

                    Ok(None)
                }
            }

            // unsupported builtin module specifier
            _ => {
                state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                });

                Ok(None)
            }
        }
    }

    /// Resolve one user package dependency module.
    fn resolve_user_dependency_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        specifier_parts: DependencySpecifier,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let builtin = self.repository.builtin_package();

        // resolve internal package specifier
        if let Some(uri) = builtin.module_uri_for_specifier(specifier) {
            self.resolve_dependency_uri(state, anchor, specifier, &uri)
        }
        // resolve source graph specifier
        else {
            match specifier_parts {
                // same package module
                DependencySpecifier::Relative(specifier_parts) => self
                    .resolve_relative_dependency_module(
                        state,
                        anchor,
                        &specifier_parts,
                        specifier,
                        loader,
                    ),

                // dependency package export
                DependencySpecifier::Package(specifier_parts) => self.resolve_dependency_export(
                    state,
                    anchor,
                    &specifier_parts,
                    specifier,
                    loader,
                ),

                // unsupported user module specifier
                DependencySpecifier::Absolute
                | DependencySpecifier::Private
                | DependencySpecifier::Internal
                | DependencySpecifier::Scheme
                | DependencySpecifier::Invalid => {
                    state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                        anchor: anchor.clone(),
                        target: specifier.to_string(),
                    });

                    Ok(None)
                }
            }
        }
    }

    /// Resolve one dependency package export.
    fn resolve_dependency_export(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &PackageSpecifier,
        target: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        // collect active declarations
        let package = self.package(state.revision, state.module.package_id)?;
        let dependencies = package.dependencies_for_conditions(state.conditions);

        // require explicit dependency declarations
        let Some(dependency) = dependencies.get(&specifier.package) else {
            state.report_diagnostic(ImportError::MissingPackageDependency {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
            });

            return Ok(None);
        };

        // resolve target package
        let Some(package) = self.resolve_dependency_package(
            state,
            anchor,
            &package,
            &specifier.package,
            dependency,
        )?
        else {
            state.report_diagnostic(ImportError::UnloadedPackageDependency {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
            });

            return Ok(None);
        };

        // select matching export
        let Some((export, export_path)) =
            self.matching_package_export(package.as_ref(), &specifier.export, state.conditions)
        else {
            state.report_diagnostic(ImportError::MissingPackageExport {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
                export: specifier.export.clone(),
            });

            return Ok(None);
        };

        // require source modules
        if export.kind != ExportKind::Module {
            state.report_diagnostic(ImportError::NonModulePackageExport {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
                export: specifier.export.clone(),
            });

            return Ok(None);
        }

        // resolve package relative export path
        let Some(package_root) = package.path.as_deref() else {
            return Err(ImportError::Internal {
                anchor: anchor.clone(),
                message: format!(
                    "dependency package '{}' has no root path",
                    specifier.package
                ),
            }
            .into());
        };
        let path = package_root.join(export_path.as_str());

        self.resolve_export_package_module(state, anchor, package.id, &path, target, loader)
    }

    /// Return the matching export and resolved package path for one export key.
    fn matching_package_export<'a>(
        &self,
        package: &'a Package,
        key: &str,
        conditions: &ConditionSet,
    ) -> Option<(&'a PackageExport, String)> {
        // exact export match
        if let Some(export) = package
            .export(key)
            .filter(|export| export.matches(conditions))
        {
            return Some((export, export.path.clone()));
        }

        self.matching_package_pattern_export(package, key, conditions)
    }

    /// Return the most specific matching pattern export for one export key.
    fn matching_package_pattern_export<'a>(
        &self,
        package: &'a Package,
        key: &str,
        conditions: &ConditionSet,
    ) -> Option<(&'a PackageExport, String)> {
        let mut best = None;

        // scan pattern export matches
        for (pattern, export) in &package.exports {
            let Some((replacement, prefix_len, suffix_len)) =
                package_export_replacement(pattern, key)
            else {
                continue;
            };
            if !export.matches(conditions) {
                continue;
            }

            let score = (prefix_len, suffix_len, pattern.len());
            let path = export.path.replace('*', replacement);

            // keep the most specific pattern
            if best
                .as_ref()
                .is_none_or(|(best_score, _, _)| score > *best_score)
            {
                best = Some((score, export, path));
            }
        }

        best.map(|(_, export, path)| (export, path))
    }

    /// Resolve one same-package relative dependency.
    fn resolve_relative_dependency_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier_parts: &ModuleSpecifier,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        // reject local query and fragment syntax
        if specifier_parts.query.is_some() || specifier_parts.fragment.is_some() {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        }

        self.resolve_package_module(state, anchor, specifier_parts.path(), specifier, loader)
    }

    /// Resolve one declared dependency package from its configured source.
    fn resolve_dependency_package(
        &self,
        state: &ImportState<'_>,
        anchor: &DiagnosticAnchor,
        current_package: &Package,
        package_name: &str,
        dependency: &Dependency,
    ) -> CompilerResult<Option<Arc<Package>>> {
        match dependency {
            // resolve workspace package by declared name
            Dependency::Workspace | Dependency::Git { .. } => {
                let package = self
                    .repository
                    .package_by_name(state.revision, package_name)
                    .map_err(|error| ImportError::Internal {
                        anchor: anchor.clone(),
                        message: format!(
                            "failed to read dependency package '{package_name}': {error}"
                        ),
                    })?;

                Ok(package)
            }

            // resolve path package relative to the importer
            Dependency::Path { path } => {
                let Some(current_root) = current_package.path.as_deref() else {
                    return Err(ImportError::Internal {
                        anchor: anchor.clone(),
                        message: format!("package importing '{package_name}' has no root path"),
                    }
                    .into());
                };
                let package_path = self.dependency_package_path(current_root, path);

                // resolve package by normalized root path
                let package = self
                    .repository
                    .package_by_path(state.revision, &package_path)
                    .map_err(|error| ImportError::Internal {
                        anchor: anchor.clone(),
                        message: format!(
                            "failed to read dependency package at '{}': {error}",
                            package_path.display()
                        ),
                    })?;

                Ok(package)
            }
        }
    }

    /// Resolve one exported package module path.
    fn resolve_export_package_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        package_id: PackageId,
        path: &Path,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let Some(candidates) =
            self.build_export_candidate_paths(path, specifier, anchor, state, loader)?
        else {
            return Ok(None);
        };
        let mut matches = Vec::new();

        // collect modules for existing candidate paths
        for path in candidates {
            let module_id = self.module_id_for_path(state.revision, &path)?;
            if let Some(module_id) = module_id {
                matches.push((path, module_id));
            }
        }

        match matches.as_slice() {
            // no module matched
            [] => {
                state.report_diagnostic(ImportError::UnresolvedModule {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                });

                Ok(None)
            }

            // exactly one module matched
            [(path, module_id)] => self.resolve_export_package_match(
                state, anchor, package_id, path, *module_id, specifier,
            ),

            // multiple modules matched
            _ => {
                let candidates = matches
                    .iter()
                    .map(|(path, _)| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                state.report_diagnostic(ImportError::AmbiguousModuleSpecifier {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                    candidates,
                });

                Ok(None)
            }
        }
    }

    /// Resolve one exported module match when it belongs to the dependency package.
    fn resolve_export_package_match(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        package_id: PackageId,
        path: &Path,
        module_id: ModuleId,
        specifier: &str,
    ) -> CompilerResult<Option<ModuleId>> {
        let module = self
            .repository
            .module(state.revision, module_id)
            .map_err(|error| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("failed to read dependency module {module_id:?}: {error}"),
            })?
            .ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("missing dependency module {module_id:?}"),
            })?;

        // reject direct imports of conditional module files
        let file_id = self.repository.file_id(path);
        if file_id != module.file_id {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        }

        // accept dependency package module
        if module.package_id == package_id {
            Ok(Some(module_id))
        }
        // reject cross package module
        else {
            state.report_diagnostic(ImportError::CrossPackageImport {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            Ok(None)
        }
    }

    /// Resolve one canonical dependency URI into a loaded module.
    fn resolve_dependency_uri(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        uri: &Uri,
    ) -> CompilerResult<Option<ModuleId>> {
        let Some(module_id) = self.module_id_for_uri(state.revision, uri)? else {
            state.report_diagnostic(ImportError::UnresolvedModule {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        };

        Ok(Some(module_id))
    }

    /// Resolve one same-package dependency module.
    fn resolve_package_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        path: &str,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let Some(matches) = self.find_package_modules(state, anchor, path, loader)? else {
            return Ok(None);
        };

        match matches.as_slice() {
            // no module matched
            [] => {
                state.report_diagnostic(ImportError::UnresolvedModule {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                });

                Ok(None)
            }

            // exactly one module matched
            [(path, module_id)] => {
                self.resolve_package_match(state, anchor, path, *module_id, specifier)
            }

            // multiple modules matched
            _ => {
                let candidates = matches
                    .iter()
                    .map(|(path, _)| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                state.report_diagnostic(ImportError::AmbiguousModuleSpecifier {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                    candidates,
                });

                Ok(None)
            }
        }
    }

    /// Resolve one matched package module when it is legal.
    fn resolve_package_match(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        path: &Path,
        module_id: ModuleId,
        specifier: &str,
    ) -> CompilerResult<Option<ModuleId>> {
        let module = self
            .repository
            .module(state.revision, module_id)
            .map_err(|error| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("failed to read dependency module {module_id:?}: {error}"),
            })?
            .ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("missing dependency module {module_id:?}"),
            })?;

        // reject direct imports of conditional module files
        let file_id = self.repository.file_id(path);
        if file_id != module.file_id {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        }

        // accept same package modules
        if module.package_id == state.module.package_id {
            Ok(Some(module_id))
        } else {
            state.report_diagnostic(ImportError::CrossPackageImport {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            Ok(None)
        }
    }

    /// Find package modules matching one relative module specifier.
    fn find_package_modules(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<Vec<(PathBuf, ModuleId)>>> {
        let Some(candidates) =
            self.build_package_candidate_paths(state, anchor, specifier, loader)?
        else {
            return Ok(None);
        };
        let mut matches = Vec::new();

        // collect modules for existing candidate paths
        for path in candidates {
            let module_id = self.module_id_for_path(state.revision, &path)?;
            if let Some(module_id) = module_id {
                matches.push((path, module_id));
            }
        }

        Ok(Some(matches))
    }

    /// Build candidate package module paths for one relative specifier.
    fn build_package_candidate_paths(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<Vec<PathBuf>>> {
        let specifier_path = Path::new(specifier);
        let current_path = state
            .module
            .path
            .as_deref()
            .ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("module {:?} has no dependency base path", state.module.id),
            })?;

        // resolve against the current module path
        let path = if specifier_path.is_absolute() {
            specifier_path.to_path_buf()
        } else {
            let parent = current_path.parent().ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("module {:?} has no dependency parent path", state.module.id),
            })?;

            parent.join(specifier_path)
        };
        let Some(path) = self.normalize_workspace_path(path) else {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        };

        Ok(Some(self.candidate_paths(path, loader)))
    }

    /// Build candidate module paths for one package export path.
    fn build_export_candidate_paths(
        &self,
        path: &Path,
        specifier: &str,
        anchor: &DiagnosticAnchor,
        state: &mut ImportState<'_>,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<Vec<PathBuf>>> {
        let Some(path) = self.normalize_workspace_path(path.to_path_buf()) else {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        };

        Ok(Some(self.candidate_paths(path, loader)))
    }

    /// Return candidate module paths in deterministic order.
    fn candidate_paths(&self, path: PathBuf, loader: Option<Loader>) -> Vec<PathBuf> {
        // explicit extension
        if path.extension().is_some() {
            vec![path]
        }
        // loader extension
        else if let Some(extension) = loader.and_then(Loader::extension) {
            vec![path.with_extension(extension)]
        }
        // source extensions
        else {
            CODE_FILE_TYPES
                .iter()
                .filter_map(FileType::extension)
                .map(|extension| path.with_extension(extension))
                .collect()
        }
    }

    /// Resolve one dependency package root path relative to the importing package.
    fn dependency_package_path(&self, current_root: &Path, path: &Path) -> PathBuf {
        // resolve configured path
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            current_root.join(path)
        };
        let mut normalized = PathBuf::new();

        // fold lexical path components
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::Normal(component) => normalized.push(component),
                Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
            }
        }

        normalized
    }
}

/// Return the replacement for one package export pattern.
fn package_export_replacement<'a>(pattern: &str, key: &'a str) -> Option<(&'a str, usize, usize)> {
    let (prefix, suffix) = pattern.split_once('*')?;
    if !key.starts_with(prefix) || !key.ends_with(suffix) {
        return None;
    }

    let start = prefix.len();
    let end = key.len().checked_sub(suffix.len())?;
    if start > end {
        return None;
    }

    Some((&key[start..end], prefix.len(), suffix.len()))
}
