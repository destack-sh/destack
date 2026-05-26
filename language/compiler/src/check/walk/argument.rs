use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{ArgumentTerm, CheckModuleState};

impl CheckModuleState {
    /// Walk one generic argument.
    pub(in crate::check) fn walk_generic_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
        generic_argument: &dir::GenericArgument,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::GenericArgument, id.id);
        match generic_argument {
            // <T>
            dir::GenericArgument::Type { value } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // ignore damaged syntax
            dir::GenericArgument::Error => {}
        };
    }

    /// Walk one runtime argument.
    pub(in crate::check) fn walk_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Argument, id.id);

        match argument {
            // f(name: value)
            dir::Argument::Named { value, .. } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // f(label = value)
            dir::Argument::Labeled { value, .. } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // f(value)
            dir::Argument::Positional { value } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // f(...value)
            dir::Argument::Spread { value, .. } => {
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // ignore damaged syntax
            dir::Argument::Error => {}
        };
    }

    /// Return generic argument terms from argument syntax.
    pub(in crate::check) fn generic_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Vec<ArgumentTerm> {
        arguments
            .iter()
            .map(|argument| self.generic_argument_term(*argument, false, tree))
            .collect()
    }

    /// Return generic argument terms against one applied owner's slots.
    pub(in crate::check) fn applied_generic_arguments(
        &mut self,
        owner: dir::GlobalSymbolId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Vec<ArgumentTerm> {
        arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let is_static = self.expects_static_generic_argument(owner, index);

                self.generic_argument_term(*argument, is_static, tree)
            })
            .collect()
    }

    /// Return one generic argument term.
    fn generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        is_static: bool,
        tree: &dir::Tree,
    ) -> ArgumentTerm {
        match tree.get(id) {
            // <T> for a static parameter
            dir::GenericArgument::Type { value } if is_static => {
                ArgumentTerm::Static(self.static_argument_variable(*value, tree))
            }
            // <...T> for a variadic static parameter
            dir::GenericArgument::SpreadType { value } if is_static => {
                ArgumentTerm::SpreadStatic(self.static_argument_variable(*value, tree))
            }
            // <T>
            dir::GenericArgument::Type { value } => {
                ArgumentTerm::Type(self.intern_local_type_variable(*value))
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => {
                ArgumentTerm::SpreadType(self.intern_local_type_variable(*value))
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                ArgumentTerm::Static(self.define_static_expression_variable(*value))
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                ArgumentTerm::SpreadStatic(self.define_static_expression_variable(*value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(self.input.module_id);
                let variable = self.intern_node_type_variable(source);

                ArgumentTerm::Type(variable)
            }
        }
    }

    /// Return whether one generic argument position expects a static term.
    fn expects_static_generic_argument(&self, owner: dir::GlobalSymbolId, index: usize) -> bool {
        let index = dir::GenericSlotIndex::new(index as u32);

        self.generic_parameters_for_owner(owner)
            .any(|(_, generic)| generic.slot().index == index && generic.is_static())
    }
}
