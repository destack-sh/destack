use destack_dir as dir;
use destack_source::ModuleId;

use crate::export::state::ExportState;
use crate::{Compiler, ExportError, ExportResult};

impl Compiler {
    /// Export entries declared by one expression.
    pub(in crate::export) fn collect_expression_exports(
        &self,
        state: &mut ExportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
        is_global: bool,
    ) -> ExportResult<()> {
        state.stats.export_expressions += 1;

        if !state.static_allows(expression_id.into_any())? {
            state.skip_static_node(expression_id.into_any());

            return Ok(());
        }

        match expression {
            // collect exports inside a global block
            dir::Expression::Declaration(declaration_id) => {
                if !state.static_allows(declaration_id.into_any())? {
                    state.skip_static_node(declaration_id.into_any());

                    return Ok(());
                }

                let declaration = state.view.get(*declaration_id);
                let dir::Declaration::Global(declaration) = declaration else {
                    return Ok(());
                };

                for expression_id in &declaration.expressions {
                    let expression = state.view.get(*expression_id);
                    self.collect_expression_exports(state, *expression_id, expression, true)?;
                }

                Ok(())
            }

            // collect ambient re-exports
            dir::Expression::Export {
                target: Some(_),
                items,
                ..
            } if is_global => {
                let items = state.static_allowed_items(items)?;

                self.collect_global_reexports(state, expression_id, &items)
            }

            // collect exports from another module
            dir::Expression::Export {
                target: Some(_),
                items,
                ..
            } => {
                let items = state.static_allowed_items(items)?;

                self.collect_reexports(state, expression_id, &items)
            }

            // reject local export clauses inside global blocks
            dir::Expression::Export {
                target: None,
                items,
                ..
            } if is_global => {
                let items = state.static_allowed_items(items)?;

                for item_id in &items {
                    state.report_diagnostic(ExportError::UnsupportedGlobalExport {
                        anchor: state.anchor_node(item_id.id)?,
                    });
                }

                Ok(())
            }

            // collect exports from this module
            dir::Expression::Export {
                target: None,
                items,
                ..
            } => {
                let items = state.static_allowed_items(items)?;

                self.collect_clause_exports(state, &items)
            }

            // ignore non-export expressions
            _ => Ok(()),
        }
    }

    /// Return one local export entry from a dependency item.
    fn clause_export_entry(
        &self,
        state: &mut ExportState<'_>,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> ExportResult<Option<dir::ExportEntry>> {
        let item = state.view.get(item_id);

        match item {
            // skip parser recovery items and default value exports
            dir::DependencyItem::Error => Ok(None),
            _ if item.is_default_value_export() => Ok(None),

            // export one local binding
            _ => {
                let Some(source_key) = item.export_source_key() else {
                    state.report_diagnostic(ExportError::MissingExportBinding {
                        anchor: state.anchor_node(item_id.id)?,
                        name: "default".to_string(),
                    });

                    return Ok(None);
                };

                let Some(source) = state.find_module_symbol(source_key) else {
                    state.report_diagnostic(ExportError::MissingExportBinding {
                        anchor: state.anchor_node(item_id.id)?,
                        name: state.static_key_text(source_key),
                    });

                    return Ok(None);
                };

                let key = match item.export_key(state.strings()) {
                    Some(key) => Ok(key),
                    None => Err(ExportError::Internal {
                        anchor: state.anchor_node(item_id.id)?,
                        module: state.view.tree().module_id,
                        message: format!("local export item {item_id:?} has no export key"),
                    }),
                }?;

                Ok(Some(dir::ExportEntry::Local(dir::LocalExportEntry {
                    key,
                    source,
                    item: Some(item_id),
                })))
            }
        }
    }

    /// Export a local export clause.
    fn collect_clause_exports(
        &self,
        state: &mut ExportState<'_>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> ExportResult<()> {
        for item_id in items {
            let export = self.clause_export_entry(state, *item_id)?;

            // insert local export if the item creates one
            if let Some(export) = export {
                let anchor = state.anchor_node(item_id.id)?;
                state.insert_export(export, anchor)?;
            }
        }

        Ok(())
    }

    /// Return one global re-export entry from a dependency item.
    fn global_reexport_entry(
        &self,
        state: &mut ExportState<'_>,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        target: Option<ModuleId>,
    ) -> ExportResult<Option<dir::IndirectGlobalEntry>> {
        let item = state.view.get(item_id);

        match item {
            // skip parser recovery items
            dir::DependencyItem::Error => Ok(None),

            // export one imported name into the global table
            _ => {
                let key = match item.export_key(state.strings()) {
                    Some(key) => Ok(key),
                    None => Err(ExportError::Internal {
                        anchor: state.anchor_node(item_id.id)?,
                        module: state.view.tree().module_id,
                        message: format!("global re-export item {item_id:?} has no export key"),
                    }),
                }?;

                let Some(key) = key.named_key() else {
                    state.report_diagnostic(ExportError::UnsupportedGlobalExport {
                        anchor: state.anchor_node(item_id.id)?,
                    });

                    return Ok(None);
                };

                let imported = match item.export_selector() {
                    Some(imported) => Ok(imported),
                    None => Err(ExportError::Internal {
                        anchor: state.anchor_node(item_id.id)?,
                        module: state.view.tree().module_id,
                        message: format!(
                            "global re-export item {item_id:?} has no export selector"
                        ),
                    }),
                }?;

                Ok(Some(dir::IndirectGlobalEntry {
                    key,
                    item: item_id,
                    target,
                    imported,
                }))
            }
        }
    }

    /// Export a global re-export clause.
    fn collect_global_reexports(
        &self,
        state: &mut ExportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> ExportResult<()> {
        if items.is_empty() {
            return Ok(());
        }

        let target = state.reexport_target(expression_id)?;
        for item_id in items {
            let item = state.view.get(*item_id);

            match item {
                // skip parser recovery items
                dir::DependencyItem::Error => {}

                // reject ambient star re-exports
                _ if matches!(item.export_selector(), Some(dir::ExportSelector::Namespace)) => {
                    state.report_diagnostic(ExportError::UnsupportedGlobalExport {
                        anchor: state.anchor_node(item_id.id)?,
                    });
                }

                // add named ambient re-export
                _ => {
                    let entry = self.global_reexport_entry(state, *item_id, target)?;

                    if let Some(entry) = entry {
                        state.globals.push_indirect(entry);
                    }
                }
            }
        }

        Ok(())
    }

    /// Return one indirect export entry from a dependency item.
    fn reexport_entry(
        &self,
        state: &ExportState<'_>,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        target: Option<ModuleId>,
    ) -> ExportResult<Option<dir::ExportEntry>> {
        let item = state.view.get(item_id);

        match item {
            // skip parser recovery items
            dir::DependencyItem::Error => Ok(None),

            // export one imported selector
            _ => {
                let key = match item.export_key(state.strings()) {
                    Some(key) => Ok(key),
                    None => Err(ExportError::Internal {
                        anchor: state.anchor_node(item_id.id)?,
                        module: state.view.tree().module_id,
                        message: format!("re-export item {item_id:?} has no export key"),
                    }),
                }?;

                let imported = match item.export_selector() {
                    Some(imported) => Ok(imported),
                    None => Err(ExportError::Internal {
                        anchor: state.anchor_node(item_id.id)?,
                        module: state.view.tree().module_id,
                        message: format!("re-export item {item_id:?} has no export selector"),
                    }),
                }?;

                Ok(Some(dir::ExportEntry::Indirect(dir::IndirectExportEntry {
                    key,
                    item: item_id,
                    target,
                    imported,
                })))
            }
        }
    }

    /// Export a re-export clause.
    fn collect_reexports(
        &self,
        state: &mut ExportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> ExportResult<()> {
        if items.is_empty() {
            return Ok(());
        }

        let target = state.reexport_target(expression_id)?;
        for item_id in items {
            let item = state.view.get(*item_id);

            match item {
                // skip parser recovery items
                dir::DependencyItem::Error => {}

                // append star re-export
                _ if item.is_star_export() => {
                    let star_export = dir::StarExportEntry {
                        item: *item_id,
                        target,
                    };

                    state.exports.push_star(star_export);
                }

                // insert named re-export
                _ => {
                    let export = self.reexport_entry(state, *item_id, target)?;

                    if let Some(export) = export {
                        let anchor = state.anchor_node(item_id.id)?;
                        state.insert_export(export, anchor)?;
                    }
                }
            }
        }

        Ok(())
    }
}
