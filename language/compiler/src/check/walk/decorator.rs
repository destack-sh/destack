use destack_dir as dir;

use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Walk one decorator.
    ///
    /// Example:
    /// ```ds
    /// @inline
    /// function f() {}
    /// ```
    pub(in crate::check) fn walk_decorator(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        // check decorator operand outside owner flow
        let before_decorator = self.fork_flow();

        self.walk_expression(tree, decorator.expression, tree.get(decorator.expression));
        self.restore_flow(before_decorator);
    }
}
