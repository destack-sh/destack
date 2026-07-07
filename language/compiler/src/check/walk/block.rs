use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Expectation, PlaceUse, WalkState};

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
        result: Option<Expectation>,
    ) -> CompilerResult<()> {
        if !self.decide_decorated_presence(id.into_any())? {
            return Ok(());
        }
        self.enter_node(id)?;

        // walk leading statements
        let mut is_reachable = true;
        let mut warned_unreachable = false;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_value_expression(*expression, PlaceUse::Read)?;
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
                self.walk_value_expression(*expression, PlaceUse::Read)?;
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
        if block.context == dir::BlockContext::Expression {
            // blocks that cannot reach their end never produce a value
            if !is_reachable {
                let never = self.intern_type(dir::Type::Never)?;
                self.commit_node_type(id, never)?;
            }
            // queue checked blocks through their owner
            else if let Some(result) = result {
                self.queue_node_check(id, result)?;
            } else {
                self.queue_node_task(id, PlaceUse::Read)?;
            }
        } else {
            let void = self.intern_type(dir::Type::Void)?;
            self.commit_node_type(id, void)?;
        }

        Ok(())
    }
}
