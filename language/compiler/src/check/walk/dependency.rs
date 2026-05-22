use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one dependency item and collect check work.
    pub(in crate::check) fn walk_dependency_item(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        dir::walk_dependency_item(self, tree, id, dependency_item);
    }
}
