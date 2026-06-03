use destack_dir as dir;

use crate::check::{
    FunctionParameter, FunctionTerm, GenericArgument, GenericSlotId, Origin, ReceiverCapture,
    TermId, TypeOperand, TypeRelation, TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Return one function type term from a function signature.
    ///
    /// Example:
    /// ```ds
    /// function run<T>(value: T): T { value }
    /// ```
    pub(in crate::check) fn lower_function_signature_term(
        &mut self,
        signature: &dir::FunctionSignature,
        receiver_type: Option<TypeOperand>,
        return_type: Option<TypeOperand>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect generic and receiver parameters
        let mut generic_parameters = signature
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.generic_slot(*parameter, tree))
            .collect::<Vec<_>>();
        generic_parameters.extend(
            signature
                .parameters
                .iter()
                .filter_map(|parameter| self.comptime_parameter_slot(*parameter, tree)),
        );
        let this_parameter = signature
            .this_parameter
            .and_then(|parameter| self.parameter_type(parameter, tree))
            .or(receiver_type);

        // collect runtime parameters
        let parameters = signature
            .parameters
            .iter()
            .filter_map(|parameter| self.lower_function_parameter_term(*parameter, tree))
            .collect();

        let function = FunctionTerm {
            asynchrony: signature.asynchrony,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
        };

        self.check.inference.push_term(function)
    }

    /// Return one function type term from a type-space function declaration.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    pub(in crate::check) fn lower_function_type_term(
        &mut self,
        declaration: &dir::FunctionTypeExpression,
        return_type: Option<TypeOperand>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect generic and receiver parameters
        let mut generic_parameters = declaration
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.generic_slot(*parameter, tree))
            .collect::<Vec<_>>();
        generic_parameters.extend(
            declaration
                .parameters
                .iter()
                .filter_map(|parameter| self.comptime_parameter_slot(*parameter, tree)),
        );
        let this_parameter = declaration
            .this_parameter
            .and_then(|parameter| self.parameter_type(parameter, tree));

        // collect runtime parameters
        let parameters = declaration
            .parameters
            .iter()
            .filter_map(|parameter| self.lower_function_parameter_term(*parameter, tree))
            .collect();

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
        };

        self.check.inference.push_term(function)
    }

    /// Return one function type term from a type-space constructor declaration.
    ///
    /// Example:
    /// ```ds
    /// new (value: T) => Box<T>
    /// ```
    pub(in crate::check) fn lower_constructor_type_term(
        &mut self,
        declaration: &dir::ConstructorType,
        return_type: Option<TypeOperand>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect constructor generic parameters
        let mut generic_parameters = declaration
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.generic_slot(*parameter, tree))
            .collect::<Vec<_>>();
        generic_parameters.extend(
            declaration
                .parameters
                .iter()
                .filter_map(|parameter| self.comptime_parameter_slot(*parameter, tree)),
        );

        // collect constructor runtime parameters
        let parameters = declaration
            .parameters
            .iter()
            .filter_map(|parameter| self.lower_function_parameter_term(*parameter, tree))
            .collect();

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
        };

        self.check.inference.push_term(function)
    }

    /// Walk one function body inside a function flow frame.
    ///
    /// Example:
    /// ```ds
    /// function run(value: number): number {
    ///     return value;
    /// }
    /// ```
    pub(in crate::check) fn walk_function_body(
        &mut self,
        tree: &dir::Tree,
        symbol: dir::GlobalSymbolId,
        signature: &dir::FunctionSignature,
        body: dir::LocalNodeId<dir::Expression>,
        result: TypeOperand,
        receiver: Option<ReceiverCapture>,
    ) {
        let mut return_target = result;
        let mut yield_target = None;
        let mut resume_target = None;

        // allocate async result channel
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let source = body.into_global_any(tree.module_id);
            let origin = Origin::Node(source);
            let completed = self.check.create_type_variable(tree.module_id, origin);
            let symbol = self
                .check
                .language_symbol(tree.module_id, dir::LanguageItem::Promise);
            let argument = GenericArgument::Type(completed.into());
            let promised = TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments: vec![argument].into(),
            };
            let promised = self.check.inference.push_term(promised);
            let condition = self.active_static_guard();

            self.check.relate_type(
                origin,
                TypeRelation::Assignable,
                promised,
                result,
                condition,
            );
            return_target = completed.into();
        }

        // allocate generator channels
        if signature.is_generator {
            let source = body.into_global_any(tree.module_id);
            let origin = Origin::Node(source);
            let yielded = self.check.create_type_variable(tree.module_id, origin);
            let completed = self.check.create_type_variable(tree.module_id, origin);
            let resumed = self.check.create_type_variable(tree.module_id, origin);
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };

            let symbol = self.check.language_symbol(tree.module_id, item);
            let yielded_argument = GenericArgument::Type(yielded.into());
            let completed_argument = GenericArgument::Type(completed.into());
            let resumed_argument = GenericArgument::Type(resumed.into());
            let generated = TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments: vec![yielded_argument, completed_argument, resumed_argument].into(),
            };
            let generated = self.check.inference.push_term(generated);
            let condition = self.active_static_guard();

            self.check.relate_type(
                origin,
                TypeRelation::Assignable,
                generated,
                result,
                condition,
            );

            return_target = completed.into();
            yield_target = Some(yielded);
            resume_target = Some(resumed);
        }

        // enter function flow
        self.enter_function_frame(
            symbol,
            return_target,
            yield_target,
            resume_target,
            signature.asynchrony,
            receiver,
        );

        // mark entry bindings as definitely assigned
        if let Some(parameter) = signature.this_parameter {
            self.mark_bindings_assigned(tree, parameter.into_any());
        }
        for parameter in &signature.parameters {
            self.mark_bindings_assigned(tree, parameter.into_any());
        }

        // walk body and constrain implicit return
        self.walk_expression(tree, body, tree.get(body));
        if !Self::is_constructor_signature(signature)
            && self.expression_can_complete_normally(tree, body)
        {
            self.constrain_function_completion_return(tree, body);
        }

        self.leave_function_frame();
    }

    /// Return whether one signature is a constructor body.
    fn is_constructor_signature(signature: &dir::FunctionSignature) -> bool {
        matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        )
    }

    /// Return one runtime function parameter term.
    ///
    /// Example:
    /// ```ds
    /// (value?: T, ...rest: U[])
    /// ```
    fn lower_function_parameter_term(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<FunctionParameter> {
        let parameter = tree.get(id);
        let ty = self.parameter_type(id, tree)?;

        // optionality is only encoded on non variadic parameters
        let is_optional = match parameter {
            // (name?: T)
            dir::Parameter::Named { is_optional, .. }
            // ({ name }?: T)
            | dir::Parameter::Pattern { is_optional, .. } => *is_optional,
            // (...name: T)
            dir::Parameter::VariadicNamed { .. }
            // (...{ name }: T)
            | dir::Parameter::VariadicPattern { .. }
            // ignore damaged syntax
            | dir::Parameter::Error => false,
        };

        // rest parameters are variadic
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );

        let parameter = FunctionParameter {
            ty,
            static_slot: if parameter.is_comptime() {
                let source = id.into_any();
                let symbol = self
                    .check
                    .module(tree.module_id)
                    .declaration_symbol(source)?;

                self.check.inference.generic_slot_id_for_symbol(symbol)
            } else {
                None
            },
            is_optional,
            is_rest,
        };

        Some(parameter)
    }

    /// Return the slot bound to one generic parameter.
    fn generic_slot(
        &mut self,
        id: dir::LocalNodeId<dir::GenericParameter>,
        tree: &dir::Tree,
    ) -> Option<GenericSlotId> {
        let source = id.into_any();
        let symbol = self
            .check
            .module(tree.module_id)
            .declaration_symbol(source)?;
        let slot = self
            .check
            .inference
            .generic_slot_id_for_symbol(symbol)
            .unwrap_or_else(|| panic!("generic parameter {symbol:?} was not bound before use"));

        Some(slot)
    }

    /// Return the slot bound to one comptime runtime parameter.
    fn comptime_parameter_slot(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<GenericSlotId> {
        let parameter = tree.get(id);
        if !parameter.is_comptime() {
            return None;
        }
        let source = id.into_any();
        let symbol = self
            .check
            .module(tree.module_id)
            .declaration_symbol(source)?;
        let slot = self
            .check
            .inference
            .generic_slot_id_for_symbol(symbol)
            .unwrap_or_else(|| panic!("comptime parameter {symbol:?} was not bound before use"));

        Some(slot)
    }

    /// Return one runtime parameter type operand.
    ///
    /// Missing annotations use the parameter node variable so contextual lambdas can still receive
    /// an expected type.
    ///
    /// Example:
    /// ```ds
    /// (value: T)
    /// ```
    pub(in crate::check) fn parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<TypeOperand> {
        let declared_type = match tree.get(id) {
            dir::Parameter::Error => return None,
            parameter => parameter.declared_type(),
        };
        let Some(declared_type) = declared_type else {
            let node = id.into_global_any(tree.module_id);
            if let Some(ty) = self.check.inputs.node_type(node) {
                return Some(ty);
            }
            let variable = self.allocate_node_type_variable(id);

            return Some(variable.into());
        };

        let source = id.into_global_any(tree.module_id);
        let operand = self.allocate_node_type_operand(declared_type);
        let condition = self.active_static_guard();
        let operand = self.induce_transparent_type_operand(source, operand, condition);

        Some(operand)
    }
}
