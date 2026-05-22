use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::resolve::resolve::ExportLookup;
use crate::resolve::state::{DependencyClause, ResolveState};

impl ResolveState<'_> {
    /// Resolve recorded import and re-export clauses.
    pub(in crate::resolve) fn resolve_dependency_clauses(&mut self) -> CompilerResult<()> {
        let clauses = std::mem::take(&mut self.dependency_clauses);

        for clause in clauses {
            match clause {
                DependencyClause::Import {
                    expression_id,
                    items,
                } => {
                    self.resolve_import_expression(expression_id, items.as_deref())?;
                }
                DependencyClause::ReExport {
                    expression_id,
                    items,
                } => {
                    self.resolve_reexport_expression(expression_id, &items)?;
                }
            }
        }

        Ok(())
    }

    /// Resolve targets for one import declaration.
    fn resolve_import_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
    ) -> CompilerResult<()> {
        let source = expression_id.into_global_any(self.module);

        let Some(edge) = self
            .dependencies
            .edge_for_source(source, dir::DependencyRelation::Import)
        else {
            return Ok(());
        };
        let Some(target) = edge.target else {
            return Ok(());
        };
        let specifier = edge.specifier;
        self.imports.push_dependency(target);

        if let Some(items) = items {
            self.resolve_import_items(target, specifier, items)?;
        }

        Ok(())
    }

    /// Resolve targets for named import clause items.
    fn resolve_import_items(
        &mut self,
        target: ModuleId,
        specifier: dir::StringId,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<()> {
        for item_id in items {
            self.resolve_import_item(target, specifier, *item_id)?;
        }

        Ok(())
    }

    /// Resolve the target for one named import clause item.
    fn resolve_import_item(
        &mut self,
        target: ModuleId,
        specifier: dir::StringId,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> CompilerResult<()> {
        let Some(local_symbol) = self
            .bindings
            .symbol_for_declaration(item_id.into_global_any(self.module))
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

        match self.resolve_export_symbol(target, key)? {
            ExportLookup::Found(symbol) => {
                self.imports
                    .insert_symbol(local_symbol, dir::ImportTarget::Symbol(symbol));
            }
            ExportLookup::Missing => self.report_missing_export(item_id, key, specifier)?,
            ExportLookup::Ambiguous => {
                self.report_ambiguous_export(item_id, key, specifier)?;
            }
        }

        Ok(())
    }

    /// Resolve targets for one re-export declaration.
    fn resolve_reexport_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<()> {
        let source = expression_id.into_global_any(self.module);

        let Some(edge) = self
            .dependencies
            .edge_for_source(source, dir::DependencyRelation::ReExport)
        else {
            return Ok(());
        };
        let Some(target) = edge.target else {
            return Ok(());
        };
        let specifier = edge.specifier;
        self.imports.push_dependency(target);

        for item_id in items {
            let item = self.view.get(*item_id);

            let Some(selector) = item.export_selector() else {
                continue;
            };

            let Some(key) = selector.selected_export_key() else {
                continue;
            };

            match self.resolve_export_symbol(target, key)? {
                ExportLookup::Found(_) => {}
                ExportLookup::Missing => {
                    self.report_missing_export(*item_id, key, specifier)?;
                }
                ExportLookup::Ambiguous => {
                    self.report_ambiguous_export(*item_id, key, specifier)?;
                }
            }
        }

        Ok(())
    }
}
