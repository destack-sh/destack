use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::resolve::resolve::{ExportLookup, ExportTarget};
use crate::resolve::state::{ModuleClause, ResolveState};

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

    /// Return exported modules required by collected module clauses.
    pub(in crate::resolve) fn module_clause_targets(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.module_clauses.iter().filter_map(|clause| {
            let source = match clause {
                ModuleClause::Import { expression_id, .. }
                | ModuleClause::ReExport { expression_id, .. } => {
                    expression_id.into_global_any(self.module)
                }
            };

            self.modules
                .edge_for_source(source, clause.relation())
                .and_then(|edge| edge.target)
        })
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
            return Ok(());
        };
        let Some(target) = edge.target else {
            return Ok(());
        };
        self.imports.push_module(target);

        if let Some(items) = items {
            self.resolve_import_items(target, edge.specifier, items)?;
        }

        Ok(())
    }

    /// Resolve targets for named import clause items.
    ///
    /// Example:
    /// ```ds
    /// import { value, type Model } from "./dep.ds";
    /// ```
    fn resolve_import_items(
        &mut self,
        target: ModuleId,
        specifier: dir::StringId,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<()> {
        for item_id in items {
            self.stats.import_items += 1;
            self.resolve_import_item(target, specifier, *item_id)?;
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
        let Some(local_symbol) = self
            .bindings
            .declaration_symbol(item_id.into_global_any(self.module))
        else {
            return Ok(());
        };
        let item = self.view.get(item_id);

        if matches!(
            item,
            dir::DependencyItem::Binding {
                binding: dir::DependencyBinding::Namespace,
                ..
            }
        ) {
            self.imports
                .insert_symbol(local_symbol, dir::ImportTarget::Namespace(target));

            return Ok(());
        }

        let Some(key) = item.import_export_key() else {
            return Ok(());
        };

        match self.resolve_export_target(target, key)? {
            ExportLookup::Found(ExportTarget::Symbol(symbol)) => {
                self.imports
                    .insert_symbol(local_symbol, dir::ImportTarget::Symbol(symbol));
            }
            ExportLookup::Found(ExportTarget::Namespace(module)) => {
                self.imports
                    .insert_symbol(local_symbol, dir::ImportTarget::Namespace(module));
            }
            ExportLookup::Ambiguous(_) => {
                self.report_ambiguous_export(item_id, key, specifier)?;
            }
            ExportLookup::Missing => self.report_missing_export(item_id, key, specifier)?,
        }

        Ok(())
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
            return Ok(());
        };
        let Some(target) = edge.target else {
            return Ok(());
        };
        let specifier = edge.specifier;
        self.imports.push_module(target);

        for item_id in items {
            self.stats.reexport_items += 1;

            let item = self.view.get(*item_id);

            let Some(selector) = item.export_selector() else {
                continue;
            };

            let Some(key) = selector.selected_export_key() else {
                continue;
            };

            match self.resolve_export_target(target, key)? {
                ExportLookup::Found(_) => {}
                ExportLookup::Ambiguous(_) => {
                    self.report_ambiguous_export(*item_id, key, specifier)?;
                }
                ExportLookup::Missing => {
                    self.report_missing_export(*item_id, key, specifier)?;
                }
            }
        }

        Ok(())
    }
}

impl ModuleClause {
    /// Return the module relation represented by this clause.
    fn relation(&self) -> dir::ModuleRelation {
        match self {
            Self::Import { .. } => dir::ModuleRelation::Import,
            Self::ReExport { .. } => dir::ModuleRelation::ReExport,
        }
    }
}
