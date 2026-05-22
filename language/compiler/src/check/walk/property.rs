use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one property and collect check work.
    pub(in crate::check) fn walk_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        dir::walk_property(self, tree, id, property);
    }

    /// Walk one member and collect check work.
    pub(in crate::check) fn walk_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        dir::walk_member(self, tree, id, member);
    }
}
