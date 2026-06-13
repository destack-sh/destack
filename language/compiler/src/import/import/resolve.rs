use std::path::{Component, Path, PathBuf};

use destack_core::closest_string;
use destack_dir as dir;
use destack_source::{
    CODE_FILE_TYPES, FileType, Loader, ModuleId, ModuleSpecifier, PackageId, Uri,
};

use crate::import::state::ImportState;
use crate::{
    Compiler, CompilerResult, DiagnosticAnchor, ImportError, diagnostic_suggestion_distance,
};

use super::specifier::{ImportSpecifier, PackageSpecifier};

impl Compiler {
    /// Import one module edge.
    pub(in crate::import) fn collect_module(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
        attributes: Option<&dir::ImportAttributeClause>,
        relation: dir::ModuleRelation,
    ) -> CompilerResult<()> {
        state.stats.specifiers += 1;

        let anchor = state.anchor_node(expression_id.id)?;
        let specifier_text = state.strings().get(specifier).to_string();

        // read loader override
        let loader = state.extract_module_loader(&anchor, attributes);

        // resolve target module
        let target = self.resolve_module(state, &anchor, &specifier_text, loader)?;

        // append module edge
        let edge = dir::ModuleEdge {
            source: expression_id.into_global_any(state.module.id),
            specifier,
            relation,
            loader,
            target,
        };

        state.push_module(edge);

        Ok(())
    }

    /// Resolve the target module for one import specifier.
    fn resolve_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let specifier_parts = ImportSpecifier::parse(specifier);

        // resolve builtin module edges in builtin package space
        if self
            .repository
            .builtin_package()
            .contains_uri(state.module.uri.as_ref())
        {
            self.resolve_builtin_module(state, anchor, specifier, specifier_parts)
        }
        // resolve user module edges through package space
        else {
            self.resolve_user_module(state, anchor, specifier, specifier_parts, loader)
        }
    }

    /// Resolve one builtin package module.
    fn resolve_builtin_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        specifier_parts: ImportSpecifier,
    ) -> CompilerResult<Option<ModuleId>> {
        let builtin = self.repository.builtin_package();

        // resolve absolute builtin specifier
        if let Some(uri) = builtin.module_uri_for_internal_specifier(specifier) {
            return self.resolve_module_uri(state, anchor, specifier, &uri);
        }

        match specifier_parts {
            // resolve relative builtin specifier
            ImportSpecifier::Relative(specifier_parts) => {
                let specifier = specifier_parts.path();
                let uri =
                    builtin.module_uri_for_relative_specifier(state.module.uri.as_ref(), specifier);

                // relative builtin module exists
                if let Some(uri) = uri {
                    self.resolve_module_uri(state, anchor, specifier, &uri)
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

    /// Resolve one user package module.
    fn resolve_user_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        specifier_parts: ImportSpecifier,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let builtin = self.repository.builtin_package();

        // resolve internal package specifier
        if let Some(uri) = builtin.module_uri_for_specifier(specifier) {
            self.resolve_module_uri(state, anchor, specifier, &uri)
        }
        // resolve source graph specifier
        else {
            match specifier_parts {
                // same package module
                ImportSpecifier::Relative(specifier_parts) => {
                    self.resolve_relative_module(state, anchor, &specifier_parts, specifier, loader)
                }

                // package export
                ImportSpecifier::Package(specifier_parts) => {
                    self.resolve_package_export(state, anchor, &specifier_parts, specifier, loader)
                }

                // unsupported user module specifier
                ImportSpecifier::Absolute
                | ImportSpecifier::Private
                | ImportSpecifier::Internal
                | ImportSpecifier::Scheme
                | ImportSpecifier::Invalid => {
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
    fn resolve_package_export(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &PackageSpecifier,
        target: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        state.stats.package_exports += 1;

        // require explicit dependency declarations
        let Some(current_package) = state.index.package(state.module.package_id) else {
            return Err(ImportError::Internal {
                anchor: anchor.clone(),
                message: format!(
                    "package {:?} is missing from dependency index",
                    state.module.package_id
                ),
            }
            .into());
        };
        let Some(dependency) = current_package.dependency(&specifier.package) else {
            state.report_diagnostic(ImportError::MissingPackageDependency {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
            });

            return Ok(None);
        };

        // require loaded dependency package
        let Some(package_id) = dependency else {
            state.report_diagnostic(ImportError::UnloadedPackageDependency {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
            });

            return Ok(None);
        };
        let Some(package) = state.index.package(package_id) else {
            return Err(ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("package {package_id:?} is missing from dependency index"),
            }
            .into());
        };

        // select matching export
        let Some(export) = package.exports.get(&specifier.export) else {
            let suggestion = closest_string(
                &specifier.export,
                package
                    .exports
                    .exact
                    .keys()
                    .map(|export| export.to_string()),
                diagnostic_suggestion_distance(&specifier.export),
            );
            state.report_diagnostic(ImportError::MissingPackageExport {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
                export: specifier.export.clone(),
                suggestion,
            });

            return Ok(None);
        };

        // require source modules
        if !export.target.is_module {
            state.report_diagnostic(ImportError::NonModulePackageExport {
                anchor: anchor.clone(),
                package: specifier.package.clone(),
                export: specifier.export.clone(),
            });

            return Ok(None);
        }

        // resolve package relative export path
        let Some(package_root) = package.root.as_deref() else {
            return Err(ImportError::Internal {
                anchor: anchor.clone(),
                message: format!(
                    "dependency package '{}' has no root path",
                    specifier.package
                ),
            }
            .into());
        };
        let path = package_root.join(export.path.as_ref());

        self.resolve_export_package_module(state, anchor, package_id, &path, target, loader)
    }

    /// Resolve one same-package relative module.
    fn resolve_relative_module(
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
        state.stats.candidates += candidates.len();

        let mut matches = Vec::new();

        // collect modules for existing candidate paths
        for path in candidates {
            state.stats.probes += 1;

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
                    suggestion: None,
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
                message: format!("failed to read imported module {module_id:?}: {error}"),
            })?
            .ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("missing imported module {module_id:?}"),
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
        // reject cross package export path
        else {
            state.report_diagnostic(ImportError::CrossPackageExport {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            Ok(None)
        }
    }

    /// Resolve one canonical URI into a loaded module.
    fn resolve_module_uri(
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
                suggestion: None,
            });

            return Ok(None);
        };

        Ok(Some(module_id))
    }

    /// Resolve one same-package module.
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
                let suggestion = self.closest_relative_module_specifier(state, specifier)?;
                state.report_diagnostic(ImportError::UnresolvedModule {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                    suggestion,
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
                message: format!("failed to read imported module {module_id:?}: {error}"),
            })?
            .ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("missing imported module {module_id:?}"),
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
            state.report_diagnostic(ImportError::CrossPackageRelativeImport {
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
        state.stats.candidates += candidates.len();

        let mut matches = Vec::new();

        // collect modules for existing candidate paths
        for path in candidates {
            state.stats.probes += 1;

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
                message: format!("module {:?} has no import base path", state.module.id),
            })?;

        // resolve against the current module path
        let path = if specifier_path.is_absolute() {
            specifier_path.to_path_buf()
        } else {
            let parent = current_path.parent().ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("module {:?} has no import parent path", state.module.id),
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

    /// Return the closest same-package module specifier visible from this import.
    fn closest_relative_module_specifier(
        &self,
        state: &ImportState<'_>,
        specifier: &str,
    ) -> CompilerResult<Option<String>> {
        let Some(current_path) = state.module.path.as_deref() else {
            return Ok(None);
        };
        let Some(current_directory) = current_path.parent() else {
            return Ok(None);
        };

        let module_ids = self
            .repository
            .package_module_ids(state.revision, state.module.package_id)
            .map_err(|error| ImportError::Internal {
                anchor: DiagnosticAnchor::from(state.module.id),
                message: format!(
                    "failed to list package modules for {:?}: {error}",
                    state.module.package_id
                ),
            })?;
        let mut candidates = Vec::new();

        // collect concrete paths for sibling package modules
        for module_id in module_ids {
            if module_id == state.module.id {
                continue;
            }

            let module = self.module(state.revision, module_id)?;
            let Some(path) = module.path.as_deref() else {
                continue;
            };
            let Some(candidate) = relative_module_specifier(current_directory, path) else {
                continue;
            };

            candidates.push(candidate);
        }

        Ok(closest_string(
            specifier,
            candidates,
            diagnostic_suggestion_distance(specifier),
        ))
    }
}

/// Return a relative module specifier from one source directory to one target path.
fn relative_module_specifier(source_directory: &Path, target: &Path) -> Option<String> {
    let source = source_directory.components().collect::<Vec<_>>();
    let target = target.components().collect::<Vec<_>>();
    let mut shared = 0;

    // find the common lexical path prefix
    while shared < source.len() && shared < target.len() && source[shared] == target[shared] {
        shared += 1;
    }

    let mut path = PathBuf::new();

    // walk up to the shared prefix
    for component in &source[shared..] {
        if !matches!(component, Component::Normal(_)) {
            return None;
        }

        path.push("..");
    }

    // walk down to the target file
    for component in &target[shared..] {
        let Component::Normal(segment) = component else {
            return None;
        };

        path.push(segment);
    }

    let mut specifier = path
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    if specifier.is_empty() {
        return None;
    }

    if !specifier.starts_with('.') {
        specifier.insert_str(0, "./");
    }

    Some(specifier)
}
