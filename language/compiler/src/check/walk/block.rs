use destack_dir as dir;

use crate::CompilerResult;
use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Walk one block.
    ///
    /// Example:
    /// ```ds
    /// {
    ///     const value = 1;
    ///     value
    /// }
    /// ```
    pub(in crate::check) fn walk_block(
        &mut self,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

        // walk leading statements
        let mut is_reachable = true;
        let mut warned_unreachable = false;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_expression(*expression, self.tree.get(*expression))?;
                is_reachable = self.expression_can_complete_normally(*expression);
            }
            // check unreachable expression in isolated flow
            else {
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

        // walk tail in its value context
        if let Some(expression) = block.tail_expression {
            // update flow through reachable tail
            if is_reachable {
                self.walk_expression(expression, self.tree.get(expression))?;
            }
            // check unreachable tail in isolated flow
            else {
                if !warned_unreachable {
                    self.check
                        .report_unreachable_code(self.module, expression.into_any());
                }
                let before = self.fork_flow();
                self.walk_expression(expression, self.tree.get(expression))?;
                self.restore_flow(before);
            }
        }

        // set block value type
        if block.context == dir::BlockContext::Expression
            && let Some(expression) = block.tail_expression
        {
            let tail = self.node_type(expression)?;
            self.declare_node_type(id, tail)?;
        } else {
            let void = self.push_type(dir::Type::Void, id.into_any())?;
            self.declare_node_type(id, void)?;
        }

        Ok(())
    }
}
