use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one dependency item.
    pub(in crate::check) fn walk_dependency_item(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::DependencyItem, id.id);

        match dependency_item {
            // import { name: value }
            dir::DependencyItem::Binding {
                value: Some(value), ..
            } => {
                // check dependency alias in declaration context
                let before_value = self.checkpoint_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
            }
            // import { name }
            dir::DependencyItem::Binding { value: None, .. } => {}
            // ignore damaged syntax
            dir::DependencyItem::Error => {}
        };
    }
}
