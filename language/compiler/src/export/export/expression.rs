use tspp_artifact::DiagnosticBuilder;
use tspp_core::find_best_match;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::export::state::ExportState;
use crate::{
    Compiler, ExportError, ExportResult, diagnostic_suggestion_distance, rename_suggestion,
};

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

            // collect ambient exports from this module
            dir::Expression::Export {
                target: None,
                items,
                ..
            } if is_global => {
                let items = state.static_allowed_items(items)?;

                self.collect_global_clause_exports(state, &items)
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
    ) -> ExportResult<Option<dir::NamedExport>> {
        let item = state.view.get(item_id);

        match item {
            // skip malformed items and default value exports
            dir::DependencyItem::Error => Ok(None),
            _ if item.is_default_value_export() => Ok(None),

            // export one local binding
            _ => {
                let Some(source_key) = item.export_source_key() else {
                    state.report_diagnostic(ExportError::MissingExportBinding {
                        anchor: state.anchor_node(item_id.id)?,
                        name: "default".to_string(),
                        suggestion: None,
                    });

                    return Ok(None);
                };

                let declarations = state.visible_declarations(source_key);
                if declarations.is_empty() {
                    let name = state.static_key_text(source_key);
                    let anchor = state.anchor_node(item_id.id)?;
                    let best = find_best_match(
                        &name,
                        state.module_scope_names(),
                        diagnostic_suggestion_distance(&name),
                    );
                    let error = ExportError::MissingExportBinding {
                        anchor: anchor.clone(),
                        name,
                        suggestion: best.as_ref().map(|best| best.candidate.clone()),
                    };

                    // plain items span the bare name, so the rename patches cleanly
                    let mut diagnostic = DiagnosticBuilder::new(error);
                    let is_plain_name =
                        matches!(item, dir::DependencyItem::Binding { alias: None, .. });
                    if is_plain_name
                        && let Some(best) = best
                        && let Some(suggestion) = rename_suggestion(&anchor, &best)
                    {
                        diagnostic = diagnostic.suggestion(suggestion);
                    }
                    state.report_diagnostic(diagnostic);

                    return Ok(None);
                }

                let key = match item.export_key(state.strings()) {
                    Some(key) => Ok(key),
                    None => Err(ExportError::Internal {
                        anchor: state.anchor_node(item_id.id)?,
                        module: state.view.tree().module_id,
                        message: format!("local export item {item_id:?} has no export key"),
                    }),
                }?;

                let declaration = state.export_declaration(item_id)?;
                let imported = match declarations.as_slice() {
                    [source] => state.import_binding(*source)?,
                    _ => None,
                };
                let declaration = declaration.or(match &imported {
                    Some(dir::ExportBinding::Import { local, .. }) => Some(*local),
                    Some(
                        dir::ExportBinding::Local { .. } | dir::ExportBinding::ReExport { .. },
                    )
                    | None => None,
                });

                // retain local declarations or the exact imported route
                let binding = match imported {
                    Some(binding) => binding,
                    None => {
                        for declaration in &declarations {
                            let form = state.symbol_form(*declaration);
                            state.exports.insert_symbol_form(*declaration, form);
                        }

                        dir::ExportBinding::Local {
                            symbols: declarations,
                        }
                    }
                };

                Ok(Some(dir::NamedExport {
                    key,
                    item: Some(item_id),
                    declaration,
                    binding,
                }))
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

    /// Export a local export clause into the global table.
    fn collect_global_clause_exports(
        &self,
        state: &mut ExportState<'_>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> ExportResult<()> {
        for item_id in items {
            let Some(export) = self.clause_export_entry(state, *item_id)? else {
                continue;
            };

            let Some(key) = export.key().named_key() else {
                state.report_diagnostic(ExportError::DefaultGlobalExport {
                    anchor: state.anchor_node(item_id.id)?,
                });

                continue;
            };

            state.globals.push(dir::GlobalEntry {
                key,
                item: export.item,
                declaration: export.declaration,
                binding: export.binding,
            });
        }

        Ok(())
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
                // skip malformed items
                dir::DependencyItem::Error => {}

                // reject keyless ambient namespace re-exports
                _ if item.is_star_export() => {
                    state.report_diagnostic(ExportError::NamespaceGlobalExport {
                        anchor: state.anchor_node(item_id.id)?,
                    });
                }

                // add named ambient re-export
                _ => {
                    let Some(export) = self.reexport_entry(state, *item_id, target)? else {
                        continue;
                    };
                    let Some(key) = export.key.named_key() else {
                        state.report_diagnostic(ExportError::DefaultGlobalExport {
                            anchor: state.anchor_node(item_id.id)?,
                        });

                        continue;
                    };

                    state.globals.push(dir::GlobalEntry {
                        key,
                        item: export.item,
                        declaration: export.declaration,
                        binding: export.binding,
                    });
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
    ) -> ExportResult<Option<dir::NamedExport>> {
        let item = state.view.get(item_id);

        match item {
            // skip malformed items
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

                Ok(Some(dir::NamedExport {
                    key,
                    item: Some(item_id),
                    declaration: state.export_declaration(item_id)?,
                    binding: dir::ExportBinding::ReExport {
                        module: target,
                        selector: imported,
                    },
                }))
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
                // skip malformed items
                dir::DependencyItem::Error => {}

                // append star re-export
                _ if item.is_star_export() => {
                    let star_export = dir::StarExport {
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
