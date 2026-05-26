use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{CheckModuleState, TypeLiteralTerm, TypeTerm};

impl CheckModuleState {
    /// Walk one block.
    pub(in crate::check) fn walk_block(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Block, id.id);

        // define expression block output
        if block.context == dir::BlockContext::Expression {
            let variable = self.intern_local_type_variable(id);
            let term = match block.tail_expression {
                Some(expression) => TypeTerm::Variable(self.intern_local_type_variable(expression)),
                None => TypeTerm::Literal(TypeLiteralTerm::Void),
            };

            self.define_type_term(variable, term);
        }

        // walk leading statements
        let mut is_reachable = true;
        for expression in &block.leading_expressions {
            // update flow through reachable expressions
            if is_reachable {
                self.walk_expression(tree, *expression, tree.get(*expression));
                is_reachable = self.expression_can_fall_through(tree, *expression);
            }
            // still check unreachable expressions without leaking their flow
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
            // still check unreachable tail without leaking its flow
            else {
                let before = self.checkpoint_flow();

                self.walk_expression(tree, expression, tree.get(expression));
                self.restore_flow(before);
            }
        }
    }
}
