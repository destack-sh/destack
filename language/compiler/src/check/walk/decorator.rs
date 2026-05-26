use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one decorator.
    pub(in crate::check) fn walk_decorator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        self.visit_any(tree, dir::NodeType::Decorator, id.id);

        // check decorator operand outside owner flow
        let before_decorator = self.checkpoint_flow();

        self.walk_expression(tree, decorator.expression, tree.get(decorator.expression));
        self.restore_flow(before_decorator);
    }
}
