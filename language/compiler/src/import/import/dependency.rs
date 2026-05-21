use std::path::{Path, PathBuf};

use destack_dir as dir;
use destack_source::{CODE_FILE_TYPES, FileType, Loader, ModuleId, ModuleSpecifier, Uri};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult, DiagnosticAnchor, ImportError};

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
        let loader = state.read_module_loader(&anchor, attributes);

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
        let builtin = self.repository.builtin_package();

        // resolve builtin module edges in builtin URI space
        if builtin.contains_uri(state.module.uri.as_ref()) {
            // builtin absolute specifier
            if let Some(uri) = builtin.module_uri_for_specifier(specifier) {
                self.resolve_dependency_uri(state, anchor, specifier, &uri)
            }
            // builtin relative specifier
            else {
                let Some(specifier_parts) =
                    self.parse_relative_module_specifier(state, anchor, specifier)
                else {
                    return Ok(None);
                };
                let specifier = specifier_parts.path();
                let uri =
                    builtin.module_uri_for_relative_specifier(state.module.uri.as_ref(), specifier);

                // relative builtin module exists
                if let Some(uri) = uri {
                    self.resolve_dependency_uri(state, anchor, specifier, &uri)
                }
                // reject relative builtin module outside the builtin package
                else {
                    state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                        anchor: anchor.clone(),
                        target: specifier.to_string(),
                    });

                    Ok(None)
                }
            }
        }
        // resolve user module edges through builtin or package module lookup
        else if let Some(uri) = builtin.module_uri_for_specifier(specifier) {
            self.resolve_dependency_uri(state, anchor, specifier, &uri)
        } else {
            let Some(specifier_parts) =
                self.parse_relative_module_specifier(state, anchor, specifier)
            else {
                return Ok(None);
            };

            self.resolve_package_module(state, anchor, specifier_parts.path(), specifier, loader)
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

    /// Parse one relative module specifier.
    fn parse_relative_module_specifier(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
    ) -> Option<ModuleSpecifier> {
        // reject non relative module specifiers
        if !specifier.starts_with("./") && !specifier.starts_with("../") {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return None;
        }
        let specifier_parts = ModuleSpecifier::parse(specifier);

        // reject local query and fragment syntax
        if specifier_parts.query.is_some() || specifier_parts.fragment.is_some() {
            state.report_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return None;
        }

        Some(specifier_parts)
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

        // use exact file paths as written
        if path.extension().is_some() {
            Ok(Some(vec![path]))
        }
        // use loader extension when explicitly selected
        else if let Some(extension) = loader.and_then(Loader::extension) {
            Ok(Some(vec![path.with_extension(extension)]))
        } else {
            // try supported code module extensions in deterministic order
            let candidates = CODE_FILE_TYPES
                .iter()
                .filter_map(FileType::extension)
                .map(|extension| path.with_extension(extension))
                .collect();

            Ok(Some(candidates))
        }
    }
}
