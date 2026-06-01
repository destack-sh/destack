use destack_dir as dir;

use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Walk one dependency item.
    ///
    /// Example:
    /// ```ds
    /// export const value: number = 1;
    /// ```
    pub(in crate::check) fn walk_dependency_item(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

        match dependency_item {
            // import { name: value }
            dir::DependencyItem::Binding {
                value: Some(value), ..
            } => {
                // check dependency alias in declaration context
                let before_value = self.fork_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
            }
            // import { name }
            dir::DependencyItem::Binding { value: None, .. } => {}
            // ignore damaged syntax
            dir::DependencyItem::Error => {}
        };

        self.pop_static_guard();
    }
}
