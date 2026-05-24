use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Walk one generic parameter.
    pub(in crate::check) fn walk_generic_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) {
        dir::walk_generic_parameter(self, tree, id, generic_parameter);
    }

    /// Walk one parameter.
    pub(in crate::check) fn walk_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        dir::walk_parameter(self, tree, id, parameter);
    }

    /// Walk one generic argument.
    pub(in crate::check) fn walk_generic_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
        generic_argument: &dir::GenericArgument,
    ) {
        dir::walk_generic_argument(self, tree, id, generic_argument);
    }

    /// Walk one tuple element.
    pub(in crate::check) fn walk_tuple_element(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TupleElement>,
        tuple_element: &dir::TupleElement,
    ) {
        dir::walk_tuple_element(self, tree, id, tuple_element);
    }

    /// Walk one runtime argument.
    pub(in crate::check) fn walk_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        dir::walk_argument(self, tree, id, argument);
    }
}
