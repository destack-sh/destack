use std::collections::HashSet;

use destack_core::StringId;
use destack_dir as dir;

/// Collect string literal ids from a DIR expression tree.
pub(crate) fn collect_expression_string_literals(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> HashSet<StringId> {
    // walk the tree and gather literal ids
    let mut collector = StringLiteralCollector::default();
    let expression = tree.get(expression_id);
    dir::NodeVisitor::visit_expression(&mut collector, tree, expression_id, expression);
    collector.literals
}

/// Collect string literal ids during expression traversal.
#[derive(Default)]
struct StringLiteralCollector {
    /// Visitor options placeholder.
    options: dir::NodeVisitorOptions,
    /// String literal ids seen in the tree.
    literals: HashSet<StringId>,
}

impl dir::NodeVisitor for StringLiteralCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(string_id),
        } = expression
        {
            self.literals.insert(*string_id);
        }

        dir::walk_expression(self, tree, id, expression);
    }
}
