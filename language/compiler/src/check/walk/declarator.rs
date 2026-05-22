use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one declarator and collect initializer work.
    pub(in crate::check) fn walk_declarator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        dir::walk_declarator(self, tree, id, declarator);
    }
}
