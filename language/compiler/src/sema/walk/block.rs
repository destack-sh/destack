use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::WalkState;

impl WalkState<'_, '_> {
    /// Walk one block.
    ///
    /// Example:
    /// ```tspp
    /// {
    ///     const value = 1;
    ///     value
    /// }
    /// ```
    pub(in crate::sema) fn walk_block(
        &mut self,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }
        self.enter_node(id)?;

        // walk leading statements
        let mut is_reachable = true;
        let mut warned_unreachable = false;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_expression(*expression, self.tree.get(*expression))?;
                is_reachable = self.expression_can_complete_normally(*expression);

                // record the statement that stops the flow
                if !is_reachable {
                    self.check.module.flows.set_diverging(expression.into_any());
                }
            }
            // check unreachable expression in isolated flow
            else {
                self.check
                    .module
                    .flows
                    .set_unreachable(expression.into_any());
                if !warned_unreachable {
                    self.check
                        .report_unreachable_code(self.module, expression.into_any());
                    warned_unreachable = true;
                }
                let before = self.fork_flow();
                self.walk_expression(*expression, self.tree.get(*expression))?;
                self.restore_flow(before);
            }
        }

        // walk tail without claiming its value type
        if let Some(expression) = block.tail_expression {
            // update flow through reachable tail
            if is_reachable {
                self.walk_expression(expression, self.tree.get(expression))?;
            }
            // check unreachable tail in isolated flow
            else {
                self.check
                    .module
                    .flows
                    .set_unreachable(expression.into_any());
                if !warned_unreachable {
                    self.check
                        .report_unreachable_code(self.module, expression.into_any());
                }
                let before = self.fork_flow();
                self.walk_expression(expression, self.tree.get(expression))?;
                self.restore_flow(before);
            }
        }

        // record unreached block ends; the checker types blocks
        if !is_reachable {
            self.check
                .module_mut(self.module)
                .unreachable_ends
                .insert(id.into_any());
        }

        Ok(())
    }
}
