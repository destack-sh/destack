use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{GenericArgument, WalkState};

impl WalkState<'_, '_> {
    /// Walk one runtime argument.
    ///
    /// Example:
    /// ```ds
    /// f(name: value, ...rest)
    /// ```
    pub(in crate::check) fn walk_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

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

        self.pop_static_guard();
    }

    /// Walk generic arguments and return their terms.
    ///
    /// Example:
    /// ```ds
    /// <T, U, comptime Size = 4>
    /// ```
    pub(in crate::check) fn walk_generic_arguments(
        &mut self,
        owner: Option<dir::GlobalSymbolId>,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> SmallVec<[GenericArgument; 2]> {
        arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| self.walk_generic_argument_term(*argument, owner, index, tree))
            .collect()
    }

    /// Walk one generic argument and return its term.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn walk_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        owner: Option<dir::GlobalSymbolId>,
        index: usize,
        tree: &dir::Tree,
    ) -> GenericArgument {
        let Some(owner) = owner else {
            return self.walk_ambiguous_generic_argument_term(id, tree);
        };
        let Some(is_static) = self.generic_argument_is_static(owner, index) else {
            return self.walk_ambiguous_generic_argument_term(id, tree);
        };

        if is_static {
            self.walk_static_generic_argument_term(id, tree)
        } else {
            self.walk_type_generic_argument_term(id, tree)
        }
    }

    /// Walk one argument for a type generic slot.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn walk_type_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        tree: &dir::Tree,
    ) -> GenericArgument {
        let module = tree.module_id;
        let term = match tree.get(id) {
            // <T>
            dir::GenericArgument::Type { value } => {
                GenericArgument::Type(self.lower_type_expression_operand(tree, *value).into())
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => {
                GenericArgument::SpreadType(self.lower_type_expression_operand(tree, *value).into())
            }
            // <type Item = T>
            dir::GenericArgument::AssociatedType { name, value } => {
                GenericArgument::AssociatedType {
                    name: *name,
                    value: self.lower_type_expression_operand(tree, *value).into(),
                }
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                GenericArgument::Static(self.walk_static_expression_operand(tree, *value))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.walk_static_expression_operand(tree, *value),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.walk_static_expression_operand(tree, *value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.allocate_node_type_operand(source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Walk one argument for a static generic slot.
    ///
    /// Example:
    /// ```ds
    /// <4>
    /// ```
    fn walk_static_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        tree: &dir::Tree,
    ) -> GenericArgument {
        let module = tree.module_id;
        let term = match tree.get(id) {
            // <T>
            dir::GenericArgument::Type { value } => {
                GenericArgument::Static(self.create_static_argument_variable(*value, tree).into())
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => GenericArgument::SpreadStatic(
                self.create_static_argument_variable(*value, tree).into(),
            ),
            // <type Item = T>
            dir::GenericArgument::AssociatedType { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.create_static_argument_variable(*value, tree).into(),
                }
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                GenericArgument::Static(self.walk_static_expression_operand(tree, *value))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.walk_static_expression_operand(tree, *value),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.walk_static_expression_operand(tree, *value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.allocate_node_type_operand(source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Walk one argument whose type or static slot is still ambiguous.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn walk_ambiguous_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        tree: &dir::Tree,
    ) -> GenericArgument {
        let module = tree.module_id;
        let term = match tree.get(id) {
            // <T>
            dir::GenericArgument::Type { value } => GenericArgument::type_or_static(
                self.lower_type_expression_operand(tree, *value).into(),
                self.create_static_argument_variable(*value, tree).into(),
            ),
            // <...T>
            dir::GenericArgument::SpreadType { value } => GenericArgument::spread_type_or_static(
                self.lower_type_expression_operand(tree, *value).into(),
                self.create_static_argument_variable(*value, tree).into(),
            ),
            // <type Item = T>
            dir::GenericArgument::AssociatedType { name, value } => {
                GenericArgument::AssociatedType {
                    name: *name,
                    value: self.lower_type_expression_operand(tree, *value).into(),
                }
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                GenericArgument::Static(self.walk_static_expression_operand(tree, *value))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.walk_static_expression_operand(tree, *value),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.walk_static_expression_operand(tree, *value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.allocate_node_type_operand(source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Return whether one generic argument position is static.
    fn generic_argument_is_static(&self, owner: dir::GlobalSymbolId, index: usize) -> Option<bool> {
        let index = dir::GenericSlotIndex::new(index as u32);
        let mut variadic = None;

        for (_, generic) in self.check.inference.generic_slots_for_owner(owner) {
            if generic.slot().index == index {
                return Some(generic.is_static());
            }

            if generic.is_variadic() && generic.slot().index <= index {
                variadic = Some(generic.is_static());
            }
        }

        variadic
    }
}
