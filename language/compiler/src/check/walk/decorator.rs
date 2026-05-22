use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one decorator and collect check work.
    pub(in crate::check) fn walk_decorator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        dir::walk_decorator(self, tree, id, decorator);
    }
}
