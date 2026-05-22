use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one declaration and collect declaration-owned check work.
    pub(in crate::check) fn walk_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        dir::walk_declaration(self, tree, id, declaration);
    }

    /// Walk one enum field and collect check work.
    pub(in crate::check) fn walk_enum_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
    ) {
        dir::walk_enum_field(self, tree, id, enum_field);
    }
}
