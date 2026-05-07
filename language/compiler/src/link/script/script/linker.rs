use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::{Compiler, LinkError, LinkResult};
use destack_artifact::{EmitFormat, ModuleOutput, ScriptOutput};
use destack_codegen_js as js;
use destack_source::{FileType, ModuleId, PackageId};
use destack_workspace::{BundleFormat, BundleMode, Target};

use super::super::plan::Plan;
use super::super::{
    ModuleSet, OutputGraph, OutputId, OutputLayout, ScriptDependencyTarget, ScriptLinker,
};
use crate::link::TargetLocation;

/// One source module and rewritten script module pair inside one output.
pub(super) type OutputModule = (ModuleId, js::Module);

#[allow(clippy::too_many_arguments)]
impl<'a> ScriptLinker<'a> {
    /// Return the local binding and source identity for one import item.
    fn import_binding(
        &self,
        module: &js::Module,
        specifier: &str,
        item: &js::DependencyItem,
    ) -> Option<(String, (String, u8, Option<String>, Option<u8>))> {
        let imported_name = item.name.map(|name| match name {
            js::Name::Identifier(name) | js::Name::String(name) => {
                module.strings.get(name).to_string()
            }
        });
        let local_binding = match item.mode {
            js::DependencyMode::Default | js::DependencyMode::Namespace => {
                let alias = item.alias?;

                module.strings.get(alias).to_string()
            }
            js::DependencyMode::Item => {
                let alias = item
                    .alias
                    .map(|alias| module.strings.get(alias).to_string());
                let imported_name = imported_name.clone()?;

                alias.unwrap_or(imported_name)
            }
        };
        let source = (
            specifier.to_string(),
            Self::import_mode_tag(item.mode),
            imported_name,
            item.kind.map(Self::import_kind_tag),
        );

        Some((local_binding, source))
    }

    /// Return one stable hashable tag for one import mode.
    fn import_mode_tag(mode: js::DependencyMode) -> u8 {
        match mode {
            js::DependencyMode::Item => 0,
            js::DependencyMode::Default => 1,
            js::DependencyMode::Namespace => 2,
        }
    }

    /// Return one stable hashable tag for one import kind.
    fn import_kind_tag(kind: js::DependencyKind) -> u8 {
        match kind {
            js::DependencyKind::Type => 0,
            js::DependencyKind::Value => 1,
        }
    }

    /// Deduplicate output-local import bindings across concatenated modules.
    pub(super) fn rewrite_output_script_imports(
        &self,
        modules: &mut [OutputModule],
    ) -> LinkResult<()> {
        let mut imported_specifiers = HashSet::<String>::new();
        let mut imported_bindings =
            HashMap::<String, (String, u8, Option<String>, Option<u8>)>::new();

        // normalize one module at a time in stable output order
        for (module_id, module) in modules.iter_mut() {
            let roots = module.roots.clone();
            let mut normalized_roots = Vec::with_capacity(roots.len());

            // normalize one import root at a time
            for root in roots {
                if root.ty != js::NodeType::Statement {
                    normalized_roots.push(root);
                    continue;
                }

                let statement_id = js::LocalNodeId::<js::Statement>::new(root.id);
                let statement = module.tree.get(statement_id).clone();

                let js::Statement::Import {
                    kind,
                    target,
                    items,
                    attributes,
                    ..
                } = statement
                else {
                    normalized_roots.push(root);
                    continue;
                };
                let items = items.unwrap_or_default();

                // keep type imports and attributed imports untouched here
                if kind == js::DependencyKind::Type || attributes.is_some() {
                    let specifier = module.strings.get(target).to_string();
                    imported_specifiers.insert(specifier);
                    normalized_roots.push(root);
                    continue;
                }

                let specifier = module.strings.get(target).to_string();

                // drop redundant side-effect imports once the specifier is already loaded
                if items.is_empty() {
                    if imported_specifiers.contains(&specifier) {
                        continue;
                    }

                    imported_specifiers.insert(specifier);
                    normalized_roots.push(root);
                    continue;
                }

                let mut kept_items = Vec::with_capacity(items.len());

                // keep only one binding declaration per output-local name
                for item_id in items {
                    let item = module.tree.get(item_id).clone();
                    let Some((local_binding, source)) =
                        self.import_binding(module, &specifier, &item)
                    else {
                        kept_items.push(item_id);
                        continue;
                    };

                    if let Some(existing) = imported_bindings.get(&local_binding) {
                        if *existing == source {
                            continue;
                        }

                        return Err(LinkError::InvalidTarget {
                            anchor: (*module_id).into(),
                            package: self.package_id,
                            target: self.target_id.clone(),
                            message: format!(
                                "linked output import binding collision for local name '{local_binding}'"
                            ),
                        });
                    }

                    imported_bindings.insert(local_binding, source);
                    kept_items.push(item_id);
                }

                // drop the whole import when every binding was already declared earlier
                if kept_items.is_empty() {
                    imported_specifiers.insert(specifier);
                    continue;
                }

                let statement = module.tree.get_mut(statement_id);

                if let js::Statement::Import { items, .. } = statement {
                    *items = Some(kept_items);
                }

                imported_specifiers.insert(specifier);
                normalized_roots.push(root);
            }

            module.roots = normalized_roots;
        }

        Ok(())
    }

    /// Load one generated script output for linking.
    pub(crate) fn script_output(&self, module_id: ModuleId) -> LinkResult<ScriptOutput> {
        let artifact = self.module_output(module_id)?;

        let ModuleOutput::Script(script) = artifact.as_ref() else {
            return Err(LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "expected script output for module {:?} target '{}'",
                    module_id,
                    self.target_name()
                ),
            });
        };

        Ok(script.as_ref().clone())
    }

    /// Return the emitted file type for one linked script target.
    pub(crate) fn script_output_file_type(&self) -> LinkResult<FileType> {
        match self.target.emit {
            EmitFormat::Js | EmitFormat::Html => Ok(FileType::JavaScript),
            EmitFormat::Ts => Ok(FileType::TypeScript),
            other => Err(LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("unsupported linked script output: {other:?}"),
            }),
        }
    }

    /// Validate that linked script printing uses one supported output format.
    pub(super) fn validate_script_print_format(&self) -> LinkResult<()> {
        // linked script printing is still esm-only
        if let Some(format) = self.target.bundle_output.format
            && format != BundleFormat::Esm
        {
            let format = match format {
                BundleFormat::Esm => "esm",
                BundleFormat::Cjs => "cjs",
                BundleFormat::Iife => "iife",
            };

            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: format!("output.format '{}' is not implemented yet", format),
            });
        }

        Ok(())
    }

    /// Build one relative import reference between two emitted outputs.
    pub(crate) fn build_script_output_import_reference(
        &self,
        target: &Target,
        from_output_id: OutputId,
        to_output_id: OutputId,
        output_layout: &OutputLayout,
        package_id: PackageId,
    ) -> LinkResult<String> {
        let target_layout = TargetLocation::new(Path::new(""), target);
        let from_output_location =
            output_layout
                .output_location(from_output_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: (package_id).into(),
                    package: package_id,
                    message: format!(
                        "missing output placement for import source output id {}",
                        from_output_id.0
                    ),
                })?;
        let to_output_location =
            output_layout
                .output_location(to_output_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: (package_id).into(),
                    package: package_id,
                    message: format!(
                        "missing output placement for import target output id {}",
                        to_output_id.0
                    ),
                })?;

        Ok(target_layout.output_reference(from_output_location, to_output_location))
    }

    /// Rewrite one linked code module inside one output graph.
    pub(super) fn rewrite_code_script_module(
        &self,
        output_id: OutputId,
        module_id: ModuleId,
        script: &ScriptOutput,
        module_set: &ModuleSet,
        output_graph: &OutputGraph,
        output_layout: &OutputLayout,
        target: &Target,
    ) -> LinkResult<js::Module> {
        match output_graph.bundle_mode() {
            BundleMode::SingleFile => self.compiler.rewrite_script_module(
                module_id,
                script,
                module_set,
                target,
                self.target_id,
                self.package_id,
                self.context,
            ),
            BundleMode::Chunked | BundleMode::PreserveModules => self.rewrite_output_script_module(
                output_id,
                module_id,
                script,
                output_graph,
                output_layout,
                target,
            ),
        }
    }

    /// Build the final linked stylesheet URL for one CSS module.
    pub(in crate::link::script) fn stylesheet_reference(
        &self,
        output_id: OutputId,
        plan: &Plan,
        module_id: ModuleId,
    ) -> LinkResult<String> {
        let output_location =
            plan.stylesheet_output_location(module_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "missing planned stylesheet output for module {:?}",
                        module_id
                    ),
                })?;
        let current_output = plan
            .output_layout()
            .output_location(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing output placement for output id {}", output_id.0),
            })?;
        let target_layout = TargetLocation::new(self.package_dir, self.target);

        Ok(target_layout.runtime_reference(current_output, output_location))
    }
}

impl Compiler {
    /// Return whether one internal re-export can be rewritten as a local export.
    pub(crate) fn can_rewrite_internal_script_reexport(
        &self,
        module: &js::Module,
        items: &[js::LocalNodeId<js::DependencyItem>],
    ) -> bool {
        items.iter().all(|item_id| {
            let item = module.tree.get(*item_id);

            item.mode == js::DependencyMode::Item && item.value.is_none()
        })
    }

    /// Build one resolved script dependency target from statement metadata.
    pub(crate) fn script_dependency_target(
        &self,
        specifier: &str,
        target_module: Option<ModuleId>,
    ) -> ScriptDependencyTarget {
        if let Some(module) = target_module {
            return ScriptDependencyTarget::Module {
                module,
                specifier: specifier.to_string(),
            };
        }

        ScriptDependencyTarget::External {
            specifier: specifier.to_string(),
        }
    }
}
