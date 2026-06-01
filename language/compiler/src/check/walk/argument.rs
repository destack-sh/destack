use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{GenericArgument, GenericSlot, Origin, VariableId, VariableKind, WalkState};

impl WalkState<'_, '_> {
    /// Walk one generic argument.
    ///
    /// Example:
    /// ```ds
    /// <T, comptime Size = 4>
    /// ```
    pub(in crate::check) fn walk_generic_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
        generic_argument: &dir::GenericArgument,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
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
            // <C>, <comptime Size = N>, <...C>
            dir::GenericArgument::Value { value }
            | dir::GenericArgument::AssociatedConst { value, .. }
            | dir::GenericArgument::SpreadValue { value } => {
                // check static argument value in static context
                let before_value = self.fork_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
            }
            // ignore damaged syntax
            dir::GenericArgument::Error => {}
        };

        self.pop_static_guard();
    }

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

    /// Lower generic argument terms from argument syntax.
    ///
    /// Example:
    /// ```ds
    /// <T, U, comptime Size = 4>
    /// ```
    pub(in crate::check) fn lower_generic_arguments(
        &mut self,
        owner: Option<dir::GlobalSymbolId>,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> SmallVec<[GenericArgument; 2]> {
        arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                self.lower_generic_argument_term(*argument, owner, index, tree)
            })
            .collect()
    }

    /// Lower explicit arguments and create omitted generic argument variables.
    ///
    /// Example:
    /// ```ds
    /// Logger
    /// ```
    pub(in crate::check) fn lower_generic_application_arguments(
        &mut self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> SmallVec<[GenericArgument; 2]> {
        let mut lowered = self.lower_generic_arguments(Some(owner), arguments, tree);
        let explicit_count = lowered.len();
        let parameters = self
            .check
            .generic_slots_for_owner(tree.module_id, owner)
            .collect::<Vec<_>>();

        // append omitted owner arguments in declaration order
        for (index, (parameter, generic)) in parameters.into_iter().enumerate() {
            if index < explicit_count {
                continue;
            }
            if generic.is_variadic() {
                continue;
            }

            lowered.push(self.lower_omitted_generic_argument(
                tree.module_id,
                source,
                parameter,
                &generic,
            ));
        }

        lowered
    }

    /// Lower one generic argument term.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn lower_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
        owner: Option<dir::GlobalSymbolId>,
        index: usize,
        tree: &dir::Tree,
    ) -> GenericArgument {
        let Some(owner) = owner else {
            return self.lower_unselected_generic_argument_term(id, tree);
        };
        let Some(parameter) = self.generic_argument_parameter(tree.module_id, owner, index) else {
            return self.lower_type_generic_argument_term(id, tree);
        };

        match parameter {
            GenericSlot::Type { .. } | GenericSlot::VariadicType { .. } => {
                self.lower_type_generic_argument_term(id, tree)
            }
            GenericSlot::Static { .. } | GenericSlot::VariadicStatic { .. } => {
                self.lower_static_generic_argument_term(id, tree)
            }
        }
    }

    /// Lower one argument for a type generic slot.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn lower_type_generic_argument_term(
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
                GenericArgument::Static(self.lower_static_expression_operand(tree, *value))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.lower_static_expression_operand(tree, *value),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.lower_static_expression_operand(tree, *value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.check.require_node_type(source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Lower one argument for a static generic slot.
    ///
    /// Example:
    /// ```ds
    /// <4>
    /// ```
    fn lower_static_generic_argument_term(
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
                GenericArgument::Static(self.lower_static_expression_operand(tree, *value))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.lower_static_expression_operand(tree, *value),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.lower_static_expression_operand(tree, *value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.check.require_node_type(source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Lower one argument whose generic owner is not selected yet.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn lower_unselected_generic_argument_term(
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
                GenericArgument::Static(self.lower_static_expression_operand(tree, *value))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.lower_static_expression_operand(tree, *value),
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.lower_static_expression_operand(tree, *value))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let source = id.into_global_any(module);
                let variable = self.check.require_node_type(source);

                GenericArgument::Type(variable.into())
            }
        };

        term
    }

    /// Return the generic parameter matched by one argument position.
    fn generic_argument_parameter(
        &self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        index: usize,
    ) -> Option<GenericSlot> {
        let index = dir::GenericSlotIndex::new(index as u32);
        let mut variadic = None;

        for (_, generic) in self.check.generic_slots_for_owner(module, owner) {
            if generic.slot().index == index {
                return Some(generic);
            }

            if generic.is_variadic() && generic.slot().index <= index {
                variadic = Some(generic);
            }
        }

        variadic
    }

    /// Lower one omitted generic argument as an inducible variable.
    fn lower_omitted_generic_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: VariableId,
        generic: &GenericSlot,
    ) -> GenericArgument {
        let kind = self.check.variable(parameter).kind;
        let variable = self
            .check
            .allocate_variable(module, kind, Origin::Node(source));

        match kind {
            VariableKind::Type => {
                self.check.induce_type_generic(
                    variable,
                    "T",
                    generic.type_constraint(),
                    dir::GenericSlotInduction::Application,
                );

                GenericArgument::Type(variable.into())
            }
            VariableKind::Static => {
                self.check.induce_static_generic(
                    variable,
                    "C",
                    generic.static_constraint(),
                    dir::GenericSlotInduction::Application,
                );

                GenericArgument::Static(variable.into())
            }
        }
    }
}
