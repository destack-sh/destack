use destack_dir as dir;

use crate::export::state::ExportState;
use crate::{Compiler, ExportResult};

impl Compiler {
    /// Collect export entries from one DIR view.
    pub(in crate::export) fn collect_exports(
        &self,
        state: &mut ExportState<'_>,
        roots: &[dir::LocalNodeId<dir::Expression>],
    ) -> ExportResult<()> {
        // record statically hidden declaration exports
        for root in roots {
            state.stats.roots += 1;

            let expression = state.view.get(*root);
            self.collect_static_hidden_declarations(state, *root, expression)?;
        }

        // collect declaration exports
        self.collect_declaration_exports(state)?;

        // scan explicit export declarations
        for root in roots {
            let expression = state.view.get(*root);
            self.collect_expression_exports(state, *root, expression, false)?;
        }

        Ok(())
    }

    /// Record declaration exports hidden by static guards.
    fn collect_static_hidden_declarations(
        &self,
        state: &mut ExportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> ExportResult<()> {
        state.stats.visibility_expressions += 1;

        if !state.static_allows(expression_id.into_any())? {
            state.skip_static_node(expression_id.into_any());
            self.hide_expression_declarations(state, expression);

            return Ok(());
        }

        match expression {
            dir::Expression::Declaration(declaration_id) => {
                if !state.static_allows(declaration_id.into_any())? {
                    state.skip_static_node(declaration_id.into_any());
                    self.hide_expression_declarations(state, expression);

                    return Ok(());
                }

                let declaration = state.view.get(*declaration_id);
                if let dir::Declaration::Global(declaration) = declaration {
                    for expression_id in &declaration.expressions {
                        let expression = state.view.get(*expression_id);
                        self.collect_static_hidden_declarations(state, *expression_id, expression)?;
                    }
                }
            }
            dir::Expression::Block(block_id) => {
                let block = state.view.get(*block_id);
                for expression_id in block.iter_expressions() {
                    let expression = state.view.get(expression_id);
                    self.collect_static_hidden_declarations(state, expression_id, expression)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Record declarations under one statically hidden expression.
    fn hide_expression_declarations(
        &self,
        state: &mut ExportState<'_>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Declaration(declaration_id) => {
                state
                    .static_hidden_declarations
                    .insert(declaration_id.into_any());

                let declaration = state.view.get(*declaration_id);
                if let dir::Declaration::Global(declaration) = declaration {
                    for expression_id in &declaration.expressions {
                        let expression = state.view.get(*expression_id);
                        self.hide_expression_declarations(state, expression);
                    }
                }
            }
            dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. } => {
                for declarator_id in declarators {
                    let declarator = state.view.get(*declarator_id);
                    state
                        .static_hidden_declarations
                        .insert(declarator.pattern.into_any());
                }
            }
            dir::Expression::Block(block_id) => {
                let block = state.view.get(*block_id);
                for expression_id in block.iter_expressions() {
                    let expression = state.view.get(expression_id);
                    self.hide_expression_declarations(state, expression);
                }
            }
            _ => {}
        }
    }
}
