use tspp_artifact::DirExported;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::export::ExportLookup;
use crate::resolve::state::ResolveState;
use crate::{CompilerError, CompilerResult};

impl ResolveState<'_> {
    /// Resolve each export item's declaration and final target.
    pub(in crate::resolve) fn resolve_export_references(
        &mut self,
        exported: &DirExported,
    ) -> CompilerResult<()> {
        let exports = exported
            .exports
            .exports()
            .filter_map(|(_, export)| {
                export
                    .item
                    .map(|item| (item, export.declaration, &export.binding))
            })
            .chain(
                exported
                    .globals
                    .entries()
                    .flat_map(|(_, entries)| entries)
                    .filter_map(|entry| {
                        entry
                            .item
                            .map(|item| (item, entry.declaration, &entry.binding))
                    }),
            );

        // resolve each export through its stored binding
        for (item, declaration, binding) in exports {
            let source = item.into_global_any(self.module);
            let (binding_declaration, target) = self.resolve_export_binding(item, binding)?;

            // prefer the local export declaration
            let declaration = match declaration {
                Some(symbol) => dir::Reference::from_symbols([symbol.into_global(self.module)]),
                None => binding_declaration,
            };
            self.references
                .insert_resolution(source, declaration, target);
        }

        Ok(())
    }

    /// Resolve one exported source item through its stored binding.
    fn resolve_export_binding(
        &mut self,
        item: dir::LocalNodeId<dir::DependencyItem>,
        binding: &dir::ExportBinding,
    ) -> CompilerResult<(dir::Reference, dir::Reference)> {
        match binding {
            // local exports select their declaration group directly
            dir::ExportBinding::Local { symbols } => {
                let target = dir::ExportTarget::symbols(
                    symbols.iter().map(|symbol| symbol.into_global(self.module)),
                )
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("local export {item:?} has no declarations"),
                })?;
                let target = dir::Reference::from(&target);

                Ok((target.clone(), target))
            }

            // exported imports reuse their exact import resolution
            dir::ExportBinding::Import { local, .. } => {
                let resolution = self.imports.symbol_resolution(*local).ok_or_else(|| {
                    CompilerError::Internal {
                        message: format!("exported import symbol {local:?} has no resolution"),
                    }
                })?;

                Ok((
                    resolution.declaration_reference(),
                    resolution.target_reference(),
                ))
            }

            // direct re-exports resolve from their stored module route
            dir::ExportBinding::ReExport { module, selector } => {
                self.stats.reexport_items += 1;
                self.resolve_reexport_binding(item, *module, *selector)
            }
        }
    }

    /// Resolve one direct re-export binding.
    fn resolve_reexport_binding(
        &mut self,
        item: dir::LocalNodeId<dir::DependencyItem>,
        module: Option<ModuleId>,
        selector: dir::ExportSelector,
    ) -> CompilerResult<(dir::Reference, dir::Reference)> {
        // unresolved module routes retain a missing reference
        let Some(module) = module else {
            return Ok((dir::Reference::Missing, dir::Reference::Missing));
        };

        // namespace exports select the target module directly
        let Some(key) = selector.selected_export_key() else {
            // the re-export item itself declares the namespace name
            let target = dir::Reference::Namespace {
                module,
                declaration: Some(item.into_global_any(self.module)),
            };

            return Ok((target.clone(), target));
        };

        // resolve named exports and report exact lookup failures
        let resolution = match self.resolve_export_target(module, key)? {
            ExportLookup::Found(resolution) => dir::ImportResolution::Resolved(resolution),
            ExportLookup::Ambiguous(resolutions) => {
                let specifier = self.reexport_specifier(item)?;
                self.report_ambiguous_export(item, key, specifier, &resolutions)?;

                dir::ImportResolution::Ambiguous(resolutions.into_iter().collect())
            }
            ExportLookup::Missing => {
                let specifier = self.reexport_specifier(item)?;
                self.report_missing_export(module, item, key, specifier)?;

                dir::ImportResolution::Missing
            }
        };

        Ok((
            resolution.declaration_reference(),
            resolution.target_reference(),
        ))
    }

    /// Return the module specifier containing one direct re-export item.
    fn reexport_specifier(
        &self,
        item: dir::LocalNodeId<dir::DependencyItem>,
    ) -> CompilerResult<dir::StringId> {
        let expression = self
            .view
            .get_parent_for(item)
            .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("re-export item {item:?} has no expression owner"),
            })?;
        let source = expression.into_global_any(self.module);
        let edge = self
            .modules
            .edge_for_source(source, dir::ModuleRelation::ReExport)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("re-export expression {expression:?} has no module edge"),
            })?;

        Ok(edge.specifier)
    }
}
