use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::resolve::state::ResolveState;

impl ResolveState<'_> {
    /// Walk active roots and collect dependency clauses.
    pub(in crate::resolve) fn walk(&mut self, roots: &[dir::LocalNodeId<dir::Expression>]) {
        let tree = self.view.tree();

        for root in roots {
            let expression = tree.get(*root);
            self.visit_expression(tree, *root, expression);
        }
    }
}

impl dir::NodeVisitor for ResolveState<'_> {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.walk_expression(tree, id, expression);
    }
}
