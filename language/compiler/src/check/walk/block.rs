use destack_dir as dir;

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
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

        // walk leading statements
        let mut is_reachable = true;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_expression(tree, *expression, tree.get(*expression));
                is_reachable = self.can_expression_fall_through(tree, *expression);
            }
            // check unreachable expression in isolated flow
            else {
                let before = self.checkpoint_flow();

                self.walk_expression(tree, *expression, tree.get(*expression));
                self.restore_flow(before);
            }
        }

        // walk tail in its value context
        if let Some(expression) = block.tail_expression {
            // update flow through reachable tail
            if is_reachable {
                self.walk_expression(tree, expression, tree.get(expression));
            }
            // check unreachable tail in isolated flow
            else {
                let before = self.checkpoint_flow();

                self.walk_expression(tree, expression, tree.get(expression));
                self.restore_flow(before);
            }
        }

        // set expression block type
        if block.context == dir::BlockContext::Expression {
            if let Some(expression) = block.tail_expression {
                let tail = self
                    .check
                    .require_local_node_type(tree.module_id, expression);

                self.output_node_type_operand(tree.module_id, id, tail);
            } else {
                let term = TypeTerm::Literal(TypeLiteralTerm::Void);

                self.output_node_type(tree.module_id, id, term);
            }
        }

        self.pop_static_guard();
    }
}
