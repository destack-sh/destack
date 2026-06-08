use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{TypeLiteralTerm, TypeTerm, WalkState};

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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        // walk leading statements
        let mut is_reachable = true;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_expression(*expression, self.tree.get(*expression))?;
                is_reachable = self.expression_can_complete_normally(*expression);
            }
            // check unreachable expression in isolated flow
            else {
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
                let before = self.fork_flow();
                self.walk_expression(expression, self.tree.get(expression))?;
                self.restore_flow(before);
            }
        }

        // set expression block type
        if block.context == dir::BlockContext::Expression {
            if let Some(expression) = block.tail_expression {
                let tail = self.node_type_operand(expression)?;
                self.bind_node_type_operand(id, tail)?;
            } else {
                let term = TypeTerm::Literal(TypeLiteralTerm::Void);
                self.bind_node_type(id, term)?;
            }
        }

        Ok(())
    }
}
