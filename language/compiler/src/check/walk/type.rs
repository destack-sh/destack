use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one type expression.
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        dir::walk_type_expression(self, tree, id, type_expression);
    }

    /// Walk one type member.
    pub(in crate::check) fn walk_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        dir::walk_type_member(self, tree, id, type_member);
    }

    /// Walk one mapped type parameter.
    pub(in crate::check) fn walk_type_mapped_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        type_mapped_parameter: &dir::TypeMappedParameter,
    ) {
        dir::walk_type_mapped_parameter(self, tree, id, type_mapped_parameter);
    }
}
