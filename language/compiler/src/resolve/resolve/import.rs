use tspp_dir as dir;
use tspp_source::{FileId, ModuleId};

use crate::export::ExportLookup;
use crate::resolve::state::ResolveState;
use crate::{CompilerError, CompilerResult};

impl ResolveState<'_> {
    /// Resolve collected import clauses.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
    /// ```
    pub(in crate::resolve) fn resolve_imports(&mut self) -> CompilerResult<()> {
        let imports = std::mem::take(&mut self.import_expressions);
        let mut seen = Vec::<(FileId, ModuleId, dir::LocalNodeId<dir::Expression>)>::new();

        // resolve each import and diagnose repeated targets within one file
        for import in imports {
            let items = match self.view.get(import) {
                dir::Expression::Import { items, .. } => items.clone(),
                expression => {
                    return Err(CompilerError::Internal {
                        message: format!("import node {import:?} has type {expression:?}"),
                    });
                }
            };
            let source = import.into_global_any(self.module);

            // require the module graph edge built for this exact import
            let Some(edge) = self
                .modules
                .edge_for_source(source, dir::ModuleRelation::Import)
            else {
                return Err(CompilerError::Internal {
                    message: format!("import expression {source:?} has no imported module edge"),
                });
            };
            let target = edge.target;
            let specifier = edge.specifier;

            // unresolved imports retain their missing item resolutions
            let Some(target) = target else {
                if let Some(items) = items.as_deref() {
                    self.record_missing_import_items(items)?;
                }

                continue;
            };

            // compare resolved targets only within the same physical source file
            let span = self.view.get_span(import);
            let duplicate = seen
                .iter()
                .find(|(file, module, _)| *file == span.file && *module == target);
            if let Some((_, _, first)) = duplicate {
                self.report_duplicate_import(import, *first, specifier)?;
            } else {
                seen.push((span.file, target, import));
            }

            // resolve every imported binding against the target exports
            if let Some(items) = items.as_deref() {
                for item in items {
                    self.stats.import_items += 1;
                    self.resolve_import_item(target, specifier, *item)?;
                }
            }
        }

        Ok(())
    }

    /// Resolve the target for one named import clause item.
    ///
    /// Example:
    /// ```tspp
    /// import { value as local } from "./dep.tspp";
    /// // local is bound to the exported value target
    /// ```
    fn resolve_import_item(
        &mut self,
        target: ModuleId,
        specifier: dir::StringId,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> CompilerResult<()> {
        let item = self.view.get(item_id);
        let source = item_id.into_global_any(self.module);

        // skip malformed item slots already rejected by parsing
        if matches!(item, dir::DependencyItem::Error) {
            return Ok(());
        }

        // require every valid import item to own its local binding
        let local_symbol =
            self.bindings
                .declaration_symbol(source)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("import dependency item {item_id:?} has no local symbol"),
                })?;

        // bind namespace imports directly to their target module
        if matches!(
            item,
            dir::DependencyItem::Binding {
                binding: dir::DependencyBinding::Namespace,
                ..
            }
        ) {
            let target = dir::ExportTarget::Namespace(target);
            let resolution = dir::ImportResolution::from(dir::ExportResolution::direct(target));
            self.record_import_resolution(local_symbol, source, resolution);

            return Ok(());
        }

        let key = item
            .import_export_key()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("import dependency item {item_id:?} has no export key"),
            })?;

        // resolve the exact import binding outcome
        let resolution = match self.resolve_export_target(target, key)? {
            ExportLookup::Found(resolution) => dir::ImportResolution::Resolved(resolution),
            ExportLookup::Ambiguous(resolutions) => {
                self.report_ambiguous_export(item_id, key, specifier, &resolutions)?;

                dir::ImportResolution::Ambiguous(resolutions.into_iter().collect())
            }
            ExportLookup::Missing => {
                self.report_missing_export(target, item_id, key, specifier)?;

                dir::ImportResolution::Missing
            }
        };
        self.record_import_resolution(local_symbol, source, resolution);

        Ok(())
    }

    /// Record one import binding resolution on its symbol and source item.
    fn record_import_resolution(
        &mut self,
        symbol: dir::LocalSymbolId,
        source: dir::GlobalNodeIdAny,
        resolution: dir::ImportResolution,
    ) {
        let target = resolution.target_reference();
        let declaration = resolution.declaration_reference();
        self.imports.insert_symbol(symbol, resolution);
        self.references
            .insert_resolution(source, declaration, target);
    }

    /// Record missing import bindings whose target module did not resolve.
    fn record_missing_import_items(
        &mut self,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<()> {
        for item in items {
            if matches!(self.view.get(*item), dir::DependencyItem::Error) {
                continue;
            }

            let source = (*item).into_global_any(self.module);
            let symbol = self.bindings.declaration_symbol(source).ok_or_else(|| {
                CompilerError::Internal {
                    message: format!("import dependency item {item:?} has no local symbol"),
                }
            })?;
            self.record_import_resolution(symbol, source, dir::ImportResolution::Missing);
        }

        Ok(())
    }
}
