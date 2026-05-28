use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{CheckState, GenericArgument};

impl CheckState<'_> {
    /// Walk one generic argument.
    pub(in crate::check) fn walk_generic_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
        generic_argument: &dir::GenericArgument,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }

        match generic_argument {
            // <T>
            dir::GenericArgument::Type { value } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // <type Item = T>
            dir::GenericArgument::AssociatedType { value, .. } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                // check static argument value in static context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { value, .. } => {
                // check static argument value in static context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                // check static argument value in static context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // ignore damaged syntax
            dir::GenericArgument::Error => {}
        };

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one runtime argument.
    pub(in crate::check) fn walk_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
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

        self.pop_static_condition(tree.module_id);
    }

    /// Build generic argument terms from argument syntax.
    pub(in crate::check) fn build_generic_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> SmallVec<[GenericArgument; 4]> {
        arguments
            .iter()
            .map(|argument| self.build_generic_argument_term(*argument, None, tree))
            .collect()
    }

    /// Build generic argument terms against one owner's slots.
    pub(in crate::check) fn build_generic_arguments_for_owner(
        &mut self,
        owner: dir::GlobalSymbolId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> SmallVec<[GenericArgument; 4]> {
        arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let is_static = self.generic_argument_is_static_for_owner(owner, index);

                self.build_generic_argument_term(*argument, Some(is_static), tree)
            })
            .collect()
    }

    /// Return one generic argument term.
    fn build_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        is_static: Option<bool>,
        tree: &dir::Tree,
    ) -> GenericArgument {
        let module = tree.module_id;
        let term = match tree.get(id) {
            // <T> for a static parameter
            dir::GenericArgument::Type { value } if is_static == Some(true) => {
                GenericArgument::Static(self.static_argument_variable(*value, tree).into())
            }
            // <...T> for a variadic static parameter
            dir::GenericArgument::SpreadType { value } if is_static == Some(true) => {
                GenericArgument::SpreadStatic(self.static_argument_variable(*value, tree).into())
            }
            // <T> with no known generic owner
            dir::GenericArgument::Type { value } if is_static.is_none() => {
                GenericArgument::TypeOrStatic {
                    ty: self.intern_local_type_variable(module, *value).into(),
                    value: self.static_argument_variable(*value, tree).into(),
                }
            }
            // <...T> with no known generic owner
            dir::GenericArgument::SpreadType { value } if is_static.is_none() => {
                GenericArgument::SpreadTypeOrStatic {
                    ty: self.intern_local_type_variable(module, *value).into(),
                    value: self.static_argument_variable(*value, tree).into(),
                }
            }
            // <T>
            dir::GenericArgument::Type { value } => {
                GenericArgument::Type(self.intern_local_type_variable(module, *value).into())
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => {
                GenericArgument::SpreadType(self.intern_local_type_variable(module, *value).into())
            }
            // <type Item = T>
            dir::GenericArgument::AssociatedType { name, value } => {
                GenericArgument::AssociatedType {
                    name: *name,
                    value: self.intern_local_type_variable(module, *value).into(),
                }
            }
            // <C>
            dir::GenericArgument::Value { value } => GenericArgument::Static(
                self.define_static_expression_variable(module, *value)
                    .into(),
            ),
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self
                        .define_static_expression_variable(module, *value)
                        .into(),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => GenericArgument::SpreadStatic(
                self.define_static_expression_variable(module, *value)
                    .into(),
            ),
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.intern_node_type_variable(module, source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Return whether one generic argument position expects a static term.
    fn generic_argument_is_static_for_owner(
        &self,
        owner: dir::GlobalSymbolId,
        index: usize,
    ) -> bool {
        let index = dir::GenericSlotIndex::new(index as u32);
        let mut variadic = None;

        for (_, generic) in self.generic_parameters_for_owner(owner) {
            if generic.slot().index == index {
                return generic.is_static();
            }

            if generic.is_variadic() && generic.slot().index <= index {
                variadic = Some(generic.is_static());
            }
        }

        variadic.unwrap_or(false)
    }
}
