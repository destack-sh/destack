use destack_dir as dir;

use crate::check::{
    FunctionParameter, FunctionTerm, GenericArgument, Origin, ReceiverCapture, TermId, TypeOperand,
    TypeRelation, TypeTerm, VariableId, VariableKind, WalkState,
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
        return_type: Option<TypeOperand>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect generic and receiver parameters
        let mut generic_parameters = signature
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.require_generic_parameter_variable(*parameter, tree))
            .collect::<smallvec::SmallVec<[VariableId; 4]>>();
        generic_parameters.extend(
            signature
                .parameters
                .iter()
                .filter_map(|parameter| self.require_comptime_parameter_variable(*parameter, tree)),
        );
        let this_parameter = signature
            .this_parameter
            .and_then(|parameter| self.ensure_parameter_type(parameter, tree));

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

        self.check.inference.terms.push(function)
    }

    /// Return one function type term from a type-space function declaration.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    pub(in crate::check) fn lower_function_type_term(
        &mut self,
        declaration: &dir::FunctionType,
        return_type: Option<TypeOperand>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect generic and receiver parameters
        let mut generic_parameters = declaration
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.require_generic_parameter_variable(*parameter, tree))
            .collect::<smallvec::SmallVec<[VariableId; 4]>>();
        generic_parameters.extend(
            declaration
                .parameters
                .iter()
                .filter_map(|parameter| self.require_comptime_parameter_variable(*parameter, tree)),
        );
        let this_parameter = declaration
            .this_parameter
            .and_then(|parameter| self.ensure_parameter_type(parameter, tree));

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

        self.check.inference.terms.push(function)
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
            .filter_map(|parameter| self.require_generic_parameter_variable(*parameter, tree))
            .collect::<smallvec::SmallVec<[VariableId; 4]>>();
        generic_parameters.extend(
            declaration
                .parameters
                .iter()
                .filter_map(|parameter| self.require_comptime_parameter_variable(*parameter, tree)),
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

        self.check.inference.terms.push(function)
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
        return_type: TypeOperand,
        receiver: Option<ReceiverCapture>,
    ) {
        let mut body_return_type = return_type;
        let mut yield_type = None;
        let mut resume_type = None;

        // allocate async result channel
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let source = body.into_global_any(tree.module_id);
            let origin = Origin::Node(source);
            let completed =
                self.check
                    .allocate_variable(tree.module_id, VariableKind::Type, origin);
            let symbol = self
                .check
                .language_symbol(tree.module_id, dir::LanguageItem::Promise);
            let argument = GenericArgument::Type(completed.into());
            let promised = TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments: vec![argument].into(),
            };
            let promised = self.check.inference.terms.push(promised);
            let condition = self.active_static_guard();

            self.check.relate_type(
                origin,
                TypeRelation::Assignable,
                promised,
                return_type,
                condition,
            );
            body_return_type = completed.into();
        }

        // allocate generator channels
        if signature.is_generator {
            let source = body.into_global_any(tree.module_id);
            let origin = Origin::Node(source);
            let yielded = self
                .check
                .allocate_variable(tree.module_id, VariableKind::Type, origin);
            let completed =
                self.check
                    .allocate_variable(tree.module_id, VariableKind::Type, origin);
            let resumed = self
                .check
                .allocate_variable(tree.module_id, VariableKind::Type, origin);
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
            let generated = self.check.inference.terms.push(generated);
            let condition = self.active_static_guard();

            self.check.relate_type(
                origin,
                TypeRelation::Assignable,
                generated,
                return_type,
                condition,
            );

            body_return_type = completed.into();
            yield_type = Some(yielded);
            resume_type = Some(resumed);
        }

        // enter function flow
        self.enter_function_frame(
            symbol,
            body_return_type,
            yield_type,
            resume_type,
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
            && self.can_expression_fall_through(tree, body)
        {
            self.constrain_function_fallthrough_return(tree, body);
        }

        self.leave_function_frame(tree.module_id);
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
        let ty = self.ensure_parameter_type(id, tree)?;

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
            is_optional,
            is_rest,
        };

        Some(parameter)
    }

    /// Return the variable bound to one generic parameter.
    fn require_generic_parameter_variable(
        &mut self,
        id: dir::LocalNodeId<dir::GenericParameter>,
        tree: &dir::Tree,
    ) -> Option<VariableId> {
        let source = id.into_any();
        let symbol = self.check.declaration_symbol(tree.module_id, source)?;
        let variable = self
            .check
            .generics
            .slots_by_symbol
            .get(&symbol)
            .copied()
            .unwrap_or_else(|| panic!("generic parameter {symbol:?} was not bound before use"));

        Some(variable)
    }

    /// Return the variable bound to one comptime runtime parameter.
    fn require_comptime_parameter_variable(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<VariableId> {
        let parameter = tree.get(id);
        if !parameter.is_comptime() {
            return None;
        }
        let source = id.into_any();
        let symbol = self.check.declaration_symbol(tree.module_id, source)?;
        let variable = self
            .check
            .generic_static_variable_for_symbol(tree.module_id, symbol)
            .unwrap_or_else(|| panic!("comptime parameter {symbol:?} was not bound before use"));

        Some(variable)
    }

    /// Ensure one runtime parameter has a checked type operand.
    ///
    /// Missing annotations use the parameter node variable so contextual lambdas can still receive
    /// an expected type.
    ///
    /// Example:
    /// ```ds
    /// (value: T)
    /// ```
    pub(in crate::check) fn ensure_parameter_type(
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
            let variable = self.check.output_node_type_variable(tree.module_id, node);

            return Some(variable.into());
        };

        Some(
            self.check
                .require_local_node_type(tree.module_id, declared_type),
        )
    }
}
