use tspp_dir as dir;

use crate::{Compiler, CompilerResult};

use super::state::ImportState;

impl Compiler {
    /// Collect module edges from one DIR view.
    pub(in crate::import) fn collect_modules(
        &self,
        state: &mut ImportState<'_>,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) -> CompilerResult<()> {
        // scan active expressions
        for root in roots {
            state.stats.roots += 1;

            let expression = state.view.get(*root);
            self.collect_expression_modules(state, *root, expression)?;
        }

        Ok(())
    }

    /// Import module edges declared by one expression.
    pub(in crate::import) fn collect_expression_modules(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        state.stats.expressions += 1;

        match expression {
            // scan module declarations inside global blocks
            dir::Expression::Declaration(declaration_id) => {
                let declaration = state.view.get(*declaration_id);
                if let dir::Declaration::Global(declaration) = declaration {
                    for expression_id in &declaration.expressions {
                        let expression = state.view.get(*expression_id);
                        self.collect_expression_modules(state, *expression_id, expression)?;
                    }
                }
            }

            // collect direct import edge
            dir::Expression::Import {
                target,
                items,
                attributes,
                ..
            } => {
                state.stats.import_clauses += 1;

                let statement_allows = state.static_allows(expression_id.into_any())?;
                let items_allow = match items {
                    Some(items) => state.static_allows_any_item(items)?,
                    None => true,
                };

                if statement_allows && items_allow {
                    self.collect_module(
                        state,
                        expression_id,
                        *target,
                        attributes.as_ref(),
                        dir::ModuleRelation::Import,
                    )?;
                } else {
                    state.stats.skipped += 1;
                }
            }

            // collect re-export edge
            dir::Expression::Export {
                target: Some(target),
                items,
                attributes,
                ..
            } => {
                state.stats.reexport_clauses += 1;

                let statement_allows = state.static_allows(expression_id.into_any())?;
                let items_allow = state.static_allows_any_item(items)?;

                if statement_allows && items_allow {
                    self.collect_module(
                        state,
                        expression_id,
                        *target,
                        attributes.as_ref(),
                        dir::ModuleRelation::ReExport,
                    )?;
                } else {
                    state.stats.skipped += 1;
                }
            }

            // ignore expressions without module declarations
            _ => {}
        }

        Ok(())
    }
}
