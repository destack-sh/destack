use destack_dir as dir;
use destack_source::ModuleId;

use crate::export::ExportLookup;
use crate::resolve::state::{ModuleClause, ResolveState};
use crate::{CompilerError, CompilerResult};

impl ResolveState<'_> {
    /// Collect import and re-export clauses from active roots.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// export { value } from "./dep.ds";
    /// ```
    pub(in crate::resolve) fn collect_module_clauses(
        &mut self,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) {
        for root in roots {
            let Some(clause) = self.module_clause_for_root(*root) else {
                continue;
            };

            self.module_clauses.push(clause);
        }
    }

    /// Resolve collected import and re-export clauses.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// export { value } from "./dep.ds";
    /// ```
    pub(in crate::resolve) fn resolve_module_clauses(&mut self) -> CompilerResult<()> {
        let clauses = std::mem::take(&mut self.module_clauses);
        for clause in clauses {
            match clause {
                ModuleClause::Import {
                    expression_id,
                    items,
                } => {
                    self.resolve_import_expression(expression_id, items.as_deref())?;
                }
                ModuleClause::ReExport {
                    expression_id,
                    items,
                } => {
                    self.resolve_reexport_expression(expression_id, &items)?;
                }
            }
        }

        Ok(())
    }

    /// Return the module clause represented by one active root.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// export { value } from "./dep.ds";
    /// ```
    fn module_clause_for_root(
        &mut self,
        root: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ModuleClause> {
        match self.view.get(root) {
            dir::Expression::Import { items, .. } => {
                self.stats.import_clauses += 1;

                Some(ModuleClause::Import {
                    expression_id: root,
                    items: items.clone(),
                })
            }

            dir::Expression::Export {
                target: Some(_),
                items,
                ..
            } => {
                self.stats.reexport_clauses += 1;

                Some(ModuleClause::ReExport {
                    expression_id: root,
                    items: items.clone(),
                })
            }

            _ => None,
        }
    }

    /// Resolve targets for one import declaration.
    ///
    /// Example:
    /// ```ds
    /// import * as dep from "./dep.ds";
    /// import { value } from "./dep.ds";
    /// ```
    fn resolve_import_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
    ) -> CompilerResult<()> {
        let source = expression_id.into_global_any(self.module);

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

        let Some(target) = target else {
            if let Some(items) = items {
                self.record_missing_import_items(items)?;
            }

            return Ok(());
        };
        if let Some(items) = items {
            for item in items {
                self.stats.import_items += 1;
                self.resolve_import_item(target, specifier, *item)?;
            }
        }

        Ok(())
    }

    /// Resolve the target for one named import clause item.
    ///
    /// Example:
    /// ```ds
    /// import { value as local } from "./dep.ds";
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
            let resolution = dir::ImportResolution::Resolved(dir::ImportTarget::Namespace(target));
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
            ExportLookup::Found(dir::ExportTarget::Symbol(symbol)) => {
                dir::ImportResolution::Resolved(dir::ImportTarget::Symbol(symbol))
            }
            ExportLookup::Found(dir::ExportTarget::Namespace(module)) => {
                dir::ImportResolution::Resolved(dir::ImportTarget::Namespace(module))
            }
            ExportLookup::Ambiguous(targets) => {
                self.report_ambiguous_export(item_id, key, specifier, &targets)?;
                let targets = targets
                    .iter()
                    .copied()
                    .map(dir::ImportTarget::from)
                    .collect();

                dir::ImportResolution::Ambiguous(targets)
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
        let reference = dir::Reference::from(&resolution);

        self.imports.insert_symbol(symbol, resolution);
        self.references.insert(source, reference);
    }

    /// Resolve targets for one re-export declaration.
    ///
    /// Example:
    /// ```ds
    /// export { value } from "./dep.ds";
    /// export * as api from "./api.ds";
    /// ```
    fn resolve_reexport_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<()> {
        let source = expression_id.into_global_any(self.module);

        let Some(edge) = self
            .modules
            .edge_for_source(source, dir::ModuleRelation::ReExport)
        else {
            return Err(CompilerError::Internal {
                message: format!("re-export expression {source:?} has no imported module edge"),
            });
        };
        let Some(target) = edge.target else {
            self.record_missing_reexport_items(items);

            return Ok(());
        };
        let specifier = edge.specifier;

        for item_id in items {
            self.stats.reexport_items += 1;
            self.resolve_reexport_item(target, specifier, *item_id)?;
        }

        Ok(())
    }

    /// Resolve the target for one re-export clause item.
    fn resolve_reexport_item(
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

        let selector = item
            .export_selector()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("re-export dependency item {item_id:?} has no selector"),
            })?;

        // namespace re-exports select the target module directly
        if selector == dir::ExportSelector::Namespace {
            self.references
                .insert(source, dir::Reference::Namespace(target));

            return Ok(());
        }

        let key = selector
            .selected_export_key()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("re-export dependency item {item_id:?} has no export key"),
            })?;

        // record the exact export lookup outcome
        match self.resolve_export_target(target, key)? {
            ExportLookup::Found(dir::ExportTarget::Symbol(symbol)) => {
                self.references
                    .insert(source, dir::Reference::from_symbols([symbol]));
            }
            ExportLookup::Found(dir::ExportTarget::Namespace(module)) => {
                self.references
                    .insert(source, dir::Reference::Namespace(module));
            }
            ExportLookup::Ambiguous(targets) => {
                let references = targets
                    .iter()
                    .copied()
                    .map(dir::ImportTarget::from)
                    .collect();
                self.references
                    .insert(source, dir::Reference::Ambiguous(references));
                self.report_ambiguous_export(item_id, key, specifier, &targets)?;
            }
            ExportLookup::Missing => {
                self.references.insert(source, dir::Reference::Missing);
                self.report_missing_export(target, item_id, key, specifier)?;
            }
        }

        Ok(())
    }

    /// Record missing re-export references whose target module did not resolve.
    fn record_missing_reexport_items(&mut self, items: &[dir::LocalNodeId<dir::DependencyItem>]) {
        for item in items {
            if matches!(self.view.get(*item), dir::DependencyItem::Error) {
                continue;
            }

            let source = (*item).into_global_any(self.module);
            self.references.insert(source, dir::Reference::Missing);
        }
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
