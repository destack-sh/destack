use std::path::{Path, PathBuf};

use destack_dir as dir;
use destack_source::{FileType, Loader, ModuleId, ModuleSpecifier};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult, DiagnosticAnchor, ImportError};

impl Compiler {
    /// Import one binding dependency edge.
    pub(in crate::import) fn collect_binding_dependency(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
        attributes: Option<&dir::ImportAttributeClause>,
        relation: dir::DependencyRelation,
    ) -> CompilerResult<()> {
        // collect the dependency edge
        self.collect_dependency(state, expression_id, specifier, attributes, relation)?;

        Ok(())
    }

    /// Import one dependency edge.
    pub(in crate::import) fn collect_dependency(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
        attributes: Option<&dir::ImportAttributeClause>,
        relation: dir::DependencyRelation,
    ) -> CompilerResult<()> {
        // resolve loader and module target
        let loader = self.loader_for_attributes(state, expression_id, attributes)?;
        let target = self.dependency_target(state, expression_id, specifier, loader)?;

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

    /// Resolve one import attribute clause into a loader override.
    fn loader_for_attributes(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        attributes: Option<&dir::ImportAttributeClause>,
    ) -> CompilerResult<Option<Loader>> {
        match attributes.and_then(|attributes| {
            attributes
                .attributes
                .iter()
                .find(|attribute| state.strings().get(attribute.key.string()) == "type")
        }) {
            // resolve the standard loader attribute
            Some(attribute) => {
                self.loader_from_attribute_value(state, expression_id, &attribute.value)
            }

            // keep the default source loader
            None => Ok(None),
        }
    }

    /// Resolve one loader type attribute value.
    fn loader_from_attribute_value(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value: &dir::ImportAttributeValue,
    ) -> CompilerResult<Option<Loader>> {
        match value {
            // resolve supported loader names
            dir::ImportAttributeValue::ScalarLiteral(dir::ScalarLiteral::String(value)) => {
                let value = state.strings().get(*value);

                if let Some(loader) = Loader::from_type_attribute(value) {
                    return Ok(Some(loader));
                }

                state.push_diagnostic(ImportError::InvalidImportAttributeType {
                    anchor: state.anchor_node(expression_id.id)?,
                    value: value.to_string(),
                });

                Ok(None)
            }

            // reject non string loader names
            _ => {
                state.push_diagnostic(ImportError::InvalidImportAttributeType {
                    anchor: state.anchor_node(expression_id.id)?,
                    value: "<non-string>".to_string(),
                });

                Ok(None)
            }
        }
    }

    /// Return the dependency target for one module specifier.
    fn dependency_target(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<ModuleId>> {
        let specifier_text = state.strings().get(specifier).to_string();
        let specifier_text = specifier_text.as_str();
        let anchor = state.anchor_node(expression_id.id)?;

        // reject non relative module specifiers
        let specifier_parts = if is_relative_specifier(specifier_text) {
            ModuleSpecifier::parse(specifier_text)
        } else {
            state.push_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor,
                target: specifier_text.to_string(),
            });

            return Ok(None);
        };

        // reject local query and fragment syntax
        if specifier_parts.query.is_some() || specifier_parts.fragment.is_some() {
            state.push_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor,
                target: specifier_text.to_string(),
            });

            return Ok(None);
        }

        // look up local modules
        let Some(matches) =
            self.candidate_dependencies(state, &anchor, specifier_parts.path(), loader)?
        else {
            return Ok(None);
        };

        match matches.as_slice() {
            // no module matched
            [] => {
                state.push_diagnostic(ImportError::UnresolvedModule {
                    anchor,
                    target: specifier_text.to_string(),
                });

                Ok(None)
            }

            // exactly one module matched
            [(path, module_id)] => {
                let module = self
                    .repository
                    .module(state.revision, *module_id)
                    .map_err(|error| ImportError::Internal {
                        anchor: anchor.clone(),
                        message: format!("failed to read dependency module {module_id:?}: {error}"),
                    })?
                    .ok_or_else(|| ImportError::Internal {
                        anchor: anchor.clone(),
                        message: format!("missing dependency module {module_id:?}"),
                    })?;

                // reject direct imports of conditional module files
                let file_id = self.repository.file_id(&path);
                if file_id != module.file_id {
                    state.push_diagnostic(ImportError::UnsupportedModuleSpecifier {
                        anchor,
                        target: specifier_text.to_string(),
                    });

                    return Ok(None);
                }

                // accept same-package local modules
                if module.package_id == state.module.package_id {
                    Ok(Some(*module_id))
                } else {
                    state.push_diagnostic(ImportError::CrossPackageImport {
                        anchor,
                        target: specifier_text.to_string(),
                    });

                    Ok(None)
                }
            }

            // multiple modules matched
            _ => {
                let candidates = matches
                    .iter()
                    .map(|(path, _)| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                state.push_diagnostic(ImportError::AmbiguousModuleSpecifier {
                    anchor,
                    target: specifier_text.to_string(),
                    candidates,
                });

                Ok(None)
            }
        }
    }

    /// Return local dependency matches for one relative module specifier.
    fn candidate_dependencies(
        &self,
        state: &mut ImportState<'_>,
        anchor: &DiagnosticAnchor,
        specifier: &str,
        loader: Option<Loader>,
    ) -> CompilerResult<Option<Vec<(PathBuf, ModuleId)>>> {
        let Some(candidates) =
            self.candidate_dependencies_paths(state, anchor, specifier, loader)?
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

    /// Return candidate local module paths for one relative specifier.
    fn candidate_dependencies_paths(
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
            state.push_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: anchor.clone(),
                target: specifier.to_string(),
            });

            return Ok(None);
        };

        // use exact file paths as written
        if path.extension().is_some() {
            return Ok(Some(vec![path]));
        }

        // use loader extension when explicitly selected
        if let Some(extension) = loader.and_then(Loader::extension) {
            return Ok(Some(vec![path.with_extension(extension)]));
        }

        // try supported code module extensions in deterministic order
        let candidates = FileType::CODE_MODULE_EXTENSION_CANDIDATES
            .iter()
            .filter_map(FileType::extension)
            .map(|extension| path.with_extension(extension))
            .collect();

        Ok(Some(candidates))
    }
}

/// Return true when a module specifier names a local relative path.
fn is_relative_specifier(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../")
}
