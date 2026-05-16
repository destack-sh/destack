use std::path::{Path, PathBuf};

use destack_dir as dir;
use destack_source::{Loader, ModuleSpecifier};

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult, ImportError};

const SOURCE_EXTENSION_CANDIDATES: &[&str] = &["ds", "d.ds", "ts", "tsx", "js", "jsx"];

impl Compiler {
    /// Import one binding dependency edge.
    pub(in crate::import) fn collect_binding_dependency(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
        attributes: Option<&dir::ImportAttributeClause>,
        relation: dir::DependencyRelation,
    ) -> CompilerResult<()> {
        // collect the dependency edge even when policy rejects the import form
        self.collect_dependency(state, expression_id, specifier, attributes, relation)?;

        // report side effect imports without blocking the dependency table
        if items.is_none() {
            state.push_diagnostic(ImportError::SideEffectImport {
                anchor: state.anchor_node(expression_id.id)?,
                target: state.strings().get(specifier).to_string(),
            });
        }

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
        let target = self.resolve_dependency_target(state, specifier, loader, expression_id)?;

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

    /// Resolve one dependency target inside the sealed repository revision.
    fn resolve_dependency_target(
        &self,
        state: &mut ImportState<'_>,
        specifier: dir::StringId,
        loader: Option<Loader>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::DependencyTarget> {
        let specifier_text = state.strings().get(specifier).to_string();
        let specifier_text = specifier_text.as_str();

        // preserve explicit host modules for the linker
        if is_protocol_specifier(specifier_text) {
            return Ok(dir::DependencyTarget::External(specifier));
        }

        // reject unsupported package and absolute forms
        if !is_relative_specifier(specifier_text) {
            state.push_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: state.anchor_node(expression_id.id)?,
                target: specifier_text.to_string(),
            });

            return Ok(dir::DependencyTarget::Unresolved);
        }

        // reject local query and fragment syntax
        let specifier_parts = ModuleSpecifier::parse(specifier_text);
        if specifier_parts.query.is_some() || specifier_parts.fragment.is_some() {
            state.push_diagnostic(ImportError::UnsupportedModuleSpecifier {
                anchor: state.anchor_node(expression_id.id)?,
                target: specifier_text.to_string(),
            });

            return Ok(dir::DependencyTarget::Unresolved);
        }

        // resolve local module paths inside the sealed revision
        let candidates =
            self.local_dependency_candidates(state, specifier_parts.path(), loader, expression_id)?;
        let mut matches = Vec::new();
        for path in candidates {
            if let Some(module_id) = self.module_id_for_path(state.revision, &path)? {
                matches.push((path, module_id));
            }
        }

        match matches.as_slice() {
            [] => {
                state.push_diagnostic(ImportError::UnresolvedModule {
                    anchor: state.anchor_node(expression_id.id)?,
                    target: specifier_text.to_string(),
                });

                Ok(dir::DependencyTarget::Unresolved)
            }
            [(_, module_id)] => {
                let anchor = state.anchor_node(expression_id.id)?;
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

                if module.package_id == state.module.package_id {
                    Ok(dir::DependencyTarget::Module(*module_id))
                } else {
                    state.push_diagnostic(ImportError::CrossPackageImport {
                        anchor: state.anchor_node(expression_id.id)?,
                        target: specifier_text.to_string(),
                    });

                    Ok(dir::DependencyTarget::Unresolved)
                }
            }
            _ => {
                let candidates = matches
                    .iter()
                    .map(|(path, _)| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                state.push_diagnostic(ImportError::AmbiguousModuleSpecifier {
                    anchor: state.anchor_node(expression_id.id)?,
                    target: specifier_text.to_string(),
                    candidates,
                });

                Ok(dir::DependencyTarget::Unresolved)
            }
        }
    }

    /// Return candidate local module paths for one relative specifier.
    fn local_dependency_candidates(
        &self,
        state: &mut ImportState<'_>,
        specifier: &str,
        loader: Option<Loader>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Vec<PathBuf>> {
        // resolve against current module path
        let anchor = state.anchor_node(expression_id.id)?;
        let specifier_path = Path::new(specifier);
        let current_path = state
            .module
            .path
            .as_deref()
            .ok_or_else(|| ImportError::Internal {
                anchor: anchor.clone(),
                message: format!("module {:?} has no dependency base path", state.module.id),
            })?;
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
                anchor,
                target: specifier.to_string(),
            });

            return Ok(Vec::new());
        };

        // use exact file paths as written
        if path.extension().is_some() {
            return Ok(vec![path]);
        }

        // use loader extension when explicitly selected
        if let Some(extension) = loader.and_then(Loader::extension) {
            return Ok(vec![path.with_extension(extension)]);
        }

        let candidates = SOURCE_EXTENSION_CANDIDATES
            .iter()
            .map(|extension| path.with_extension(extension))
            .collect();

        Ok(candidates)
    }
}

/// Return true when a module specifier names a local relative path.
fn is_relative_specifier(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../")
}

/// Return true when a module specifier uses an explicit protocol.
fn is_protocol_specifier(specifier: &str) -> bool {
    let Some((scheme, _)) = specifier.split_once(':') else {
        return false;
    };

    !scheme.is_empty()
        && scheme.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}
