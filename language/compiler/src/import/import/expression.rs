use destack_dir as dir;

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Import module edges declared by one expression.
    pub(in crate::import) fn collect_expression_modules(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        match expression {
            // scan module declarations inside global blocks
            dir::Expression::Declaration(declaration_id) => {
                let declaration = state.view.get(*declaration_id);
                if let dir::Declaration::Global(declaration) = declaration {
                    self.collect_modules(state, &declaration.expressions)?;
                }
            }

            // collect direct import edge
            dir::Expression::Import {
                target,
                items,
                attributes,
                ..
            } => {
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
                }
            }

            // collect re-export edge
            dir::Expression::Export {
                target: Some(target),
                items,
                attributes,
                ..
            } => {
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
                }
            }

            // ignore expressions without module declarations
            _ => {}
        }

        Ok(())
    }
}
