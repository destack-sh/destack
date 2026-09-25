use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::emit::js;
use crate::{Compiler, LinkError, LinkResult};
use tspp_artifact::Script;
use tspp_repository::{JsOutputMode, Target};
use tspp_source::{ModuleId, PackageId};

use super::super::{JsDependencyTarget, JsLinker, ModuleSet, OutputGraph, OutputId, OutputLayout};
use crate::link::TargetLocation;

/// One source module and rewritten JS module pair inside one output.
pub(super) type OutputModule = (ModuleId, js::Module);

impl<'a> JsLinker<'a> {
    /// Return the local binding and source identity for one import item.
    fn import_binding(
        &self,
        module: &js::Module,
        specifier: &str,
        item: &js::DependencyItem,
    ) -> Option<(String, (String, u8, Option<String>))> {
        let imported_name = item.name.map(|name| match name {
            js::Name::Identifier(name) | js::Name::String(name) => {
                module.strings.get(name).to_string()
            }
        });
        let local_binding = match item.binding {
            js::DependencyBinding::Default | js::DependencyBinding::Namespace => {
                let alias = item.alias?;

                module.strings.get(alias).to_string()
            }
            js::DependencyBinding::Named => {
                let alias = item
                    .alias
                    .map(|alias| module.strings.get(alias).to_string());
                let imported_name = imported_name.clone()?;

                alias.unwrap_or(imported_name)
            }
        };
        let source = (
            specifier.to_string(),
            Self::import_mode_tag(item.binding),
            imported_name,
        );

        Some((local_binding, source))
    }

    /// Return one stable hashable tag for one import mode.
    fn import_mode_tag(binding: js::DependencyBinding) -> u8 {
        match binding {
            js::DependencyBinding::Named => 0,
            js::DependencyBinding::Default => 1,
            js::DependencyBinding::Namespace => 2,
        }
    }

    /// Deduplicate output-local import bindings across concatenated modules.
    pub(super) fn rewrite_output_script_imports(
        &self,
        modules: &mut [OutputModule],
    ) -> LinkResult<()> {
        let mut imported_specifiers = HashSet::<String>::new();
        let mut imported_bindings = HashMap::<String, (String, u8, Option<String>)>::new();

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

                // keep attributed imports untouched here
                if attributes.is_some() {
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
                            target: *self.target_id,
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

    /// Build one relative import reference between two emitted outputs.
    pub(crate) fn build_js_output_import_reference(
        &self,
        target: &Target,
        from_output_id: OutputId,
        to_output_id: OutputId,
        output_layout: &OutputLayout,
        package_id: PackageId,
    ) -> LinkResult<String> {
        let target_layout = TargetLocation::new(Path::new(""), target, self.target_name());
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
        script: &Script,
        module_set: &ModuleSet,
        output_graph: &OutputGraph,
        output_layout: &OutputLayout,
        target: &Target,
    ) -> LinkResult<js::Module> {
        match output_graph.bundle_mode() {
            JsOutputMode::SingleFile => self.rewrite_script_module(
                module_id,
                script,
                module_set,
                target,
                self.target_id,
                self.package_id,
                self.context,
            ),
            JsOutputMode::Chunked | JsOutputMode::PreserveModules => self
                .rewrite_output_script_module(
                    output_id,
                    module_id,
                    script,
                    output_graph,
                    output_layout,
                    target,
                ),
        }
    }
}

impl Compiler {
    /// Return whether one internal re-export can be rewritten as a local export.
    pub(crate) fn can_rewrite_internal_js_reexport(
        &self,
        module: &js::Module,
        items: &[js::LocalNodeId<js::DependencyItem>],
    ) -> bool {
        items
            .iter()
            .all(|item_id| module.tree.get(*item_id).binding == js::DependencyBinding::Named)
    }

    /// Build one resolved JS dependency target from statement metadata.
    pub(crate) fn js_dependency_target(
        &self,
        specifier: &str,
        target_module: Option<ModuleId>,
    ) -> JsDependencyTarget {
        if let Some(module) = target_module {
            return JsDependencyTarget::Module {
                module,
                specifier: specifier.to_string(),
            };
        }

        JsDependencyTarget::External {
            specifier: specifier.to_string(),
        }
    }
}
