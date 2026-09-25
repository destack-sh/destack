use std::path::{Component, Path, PathBuf};

use tspp_artifact::PackageDependency;
use tspp_core::closest_string;
use tspp_dir as dir;
use tspp_source::{Loader, ModuleId, ModuleSpecifier, PackageId, Uri};

use crate::import::ModulePathOutcome;
use crate::{
    Compiler, CompilerResult, DiagnosticAnchor, ImportError, diagnostic_suggestion_distance,
};

use super::specifier::{ImportSpecifier, PackageSpecifier};
use super::state::ImportState;

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

        // resolve Builtin module edges in Builtin Package space
        if self.repository.is_builtin_package(state.module.package_id) {
            self.resolve_builtin_module(state, anchor, specifier, specifier_parts)
        }
        // resolve user module edges through package space
        else {
            self.resolve_user_module(state, anchor, specifier, specifier_parts, loader)
        }
    }

    /// Resolve one Builtin Package module.
    fn resolve_builtin_module(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        specifier_parts: ImportSpecifier,
    ) -> CompilerResult<Option<ModuleId>> {
        // resolve absolute builtin specifier
        let mut observations = Vec::new();
        let uri = self.repository.builtin_module_uri_for_internal_specifier(
            state.revision,
            specifier,
            &mut observations,
        )?;
        for observation in observations {
            state.observe(observation);
        }
        if let Some(uri) = uri {
            return self.resolve_module_uri(state, anchor, specifier, &uri);
        }

        match specifier_parts {
            // resolve relative builtin specifier
            ImportSpecifier::Relative(specifier_parts) => {
                let specifier = specifier_parts.path();
                let mut observations = Vec::new();
                let uri = self.repository.builtin_module_uri_for_relative_specifier(
                    state.revision,
                    state.module.uri.as_ref(),
                    specifier,
                    &mut observations,
                )?;
                for observation in observations {
                    state.observe(observation);
                }

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
        // resolve internal package specifier
        let mut observations = Vec::new();
        let uri = self.repository.builtin_module_uri_for_specifier(
            state.revision,
            specifier,
            &mut observations,
        )?;
        for observation in observations {
            state.observe(observation);
        }
        if let Some(uri) = uri {
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
        let current_package_id = state.module.package_id;
        let Some(current_package) = self.package_node(state, current_package_id)? else {
            return Err(ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("package {current_package_id:?} has no configuration"),
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
        let package_id = match dependency {
            // reject unavailable declared dependencies
            PackageDependency::Unavailable => {
                state.report_diagnostic(ImportError::UnloadedPackageDependency {
                    anchor: anchor.clone(),
                    package: specifier.package.clone(),
                });

                return Ok(None);
            }

            // continue through the exact dependency package
            PackageDependency::Resolved(package_id) => package_id,
        };
        let Some(package) = self.package_node(state, package_id)? else {
            return Err(ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("package {package_id:?} has no configuration"),
            }
            .into());
        };

        // select matching export; the export name lives inside the
        //  specifier string, so no rename patch applies here
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
        let resolution = self.resolve_module_path(state, path, loader)?;
        state.stats.candidates += resolution.candidates;
        state.stats.probes += resolution.candidates;

        match resolution.outcome {
            // unsupported logical path
            ModulePathOutcome::Unsupported => {
                state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                });

                Ok(None)
            }

            // no module matched
            ModulePathOutcome::Missing => {
                state.report_diagnostic(ImportError::UnresolvedModule {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                    suggestion: None,
                });

                Ok(None)
            }

            // exactly one module matched
            ModulePathOutcome::Resolved { path, module } => self
                .resolve_export_package_match(state, anchor, package_id, &path, module, specifier),

            // multiple modules matched
            ModulePathOutcome::Ambiguous { matches } => {
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
        let path = self.relative_module_path(state, anchor, path)?;
        let resolution = self.resolve_module_path(state, &path, loader)?;
        state.stats.candidates += resolution.candidates;
        state.stats.probes += resolution.candidates;

        match resolution.outcome {
            // reject paths outside the logical workspace
            ModulePathOutcome::Unsupported => {
                state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                });

                Ok(None)
            }

            // no module matched
            ModulePathOutcome::Missing => {
                let suggestion = self.closest_relative_module_specifier(state, specifier)?;
                state.report_diagnostic(ImportError::UnresolvedModule {
                    anchor: anchor.clone(),
                    target: specifier.to_string(),
                    suggestion,
                });

                Ok(None)
            }

            // exactly one module matched
            ModulePathOutcome::Resolved { path, module } => {
                self.resolve_package_match(state, anchor, &path, module, specifier)
            }

            // multiple modules matched
            ModulePathOutcome::Ambiguous { matches } => {
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

    /// Build one module path relative to the importing module.
    fn relative_module_path(
        &self,
        state: &ImportState<'_>,
        anchor: &DiagnosticAnchor,
        path: &str,
    ) -> CompilerResult<PathBuf> {
        let specifier_path = Path::new(path);
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

        Ok(path)
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

    let mut specifier = path.to_str()?.replace(std::path::MAIN_SEPARATOR, "/");
    if specifier.is_empty() {
        return None;
    }

    if !specifier.starts_with('.') {
        specifier.insert_str(0, "./");
    }

    Some(specifier)
}
