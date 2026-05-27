use destack_dir as dir;

use crate::check::CheckState;

impl CheckState<'_> {
    /// Walk one decorator.
    pub(in crate::check) fn walk_decorator(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        // check decorator operand outside owner flow
        let before_decorator = self.checkpoint_flow(tree.module_id);

        self.walk_expression(tree, decorator.expression, tree.get(decorator.expression));
        self.restore_flow(tree.module_id, before_decorator);
    }
}
