use destack_dir as dir;

use crate::check::{CheckState, TypeLiteralTerm, TypeTerm};

impl CheckState<'_> {
    /// Walk one block.
    pub(in crate::check) fn walk_block(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }

        // define expression block output
        if block.context == dir::BlockContext::Expression {
            let variable = self.intern_local_type_variable(tree.module_id, id);
            let term = match block.tail_expression {
                Some(expression) => {
                    TypeTerm::Variable(self.intern_local_type_variable(tree.module_id, expression))
                }
                None => TypeTerm::Literal(TypeLiteralTerm::Void),
            };

            self.define_type(tree.module_id, variable, term);
        }

        // walk leading statements
        let mut is_reachable = true;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_expression(tree, *expression, tree.get(*expression));
                is_reachable = self.expression_can_fall_through(tree, *expression);
            }
            // check unreachable expression in isolated flow
            else {
                let before = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *expression, tree.get(*expression));
                self.restore_flow(tree.module_id, before);
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
                let before = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, expression, tree.get(expression));
                self.restore_flow(tree.module_id, before);
            }
        }

        self.pop_static_condition(tree.module_id);
    }
}
