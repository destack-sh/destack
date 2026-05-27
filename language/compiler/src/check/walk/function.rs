use destack_dir as dir;

use crate::check::{
    CheckState, ConstraintOrigin, FunctionParameter, FunctionTerm, GenericArgument,
    ReceiverCapture, TermId, TypeRelation, TypeTerm, VariableId, VariableKind,
};

impl CheckState<'_> {
    /// Return one function type term from a function signature.
    pub(in crate::check) fn build_function_signature_term(
        &mut self,
        signature: &dir::FunctionSignature,
        return_type: Option<VariableId>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect generic and receiver parameters
        let generic_parameters = signature
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.intern_generic_parameter_variable(*parameter, tree))
            .collect();
        let this_parameter = signature
            .this_parameter
            .and_then(|parameter| self.intern_parameter_type_variable(parameter, tree));

        // collect runtime parameters
        let parameters = signature
            .parameters
            .iter()
            .filter_map(|parameter| self.build_function_parameter_term(*parameter, tree))
            .collect();

        let function = FunctionTerm {
            asynchrony: signature.asynchrony,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
        };

        self.terms.push(function)
    }

    /// Return one function type term from a type-space function declaration.
    pub(in crate::check) fn build_function_type_declaration_term(
        &mut self,
        declaration: &dir::FunctionTypeDeclaration,
        return_type: Option<VariableId>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect generic and receiver parameters
        let generic_parameters = declaration
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.intern_generic_parameter_variable(*parameter, tree))
            .collect();
        let this_parameter = declaration
            .this_parameter
            .and_then(|parameter| self.intern_parameter_type_variable(parameter, tree));

        // collect runtime parameters
        let parameters = declaration
            .parameters
            .iter()
            .filter_map(|parameter| self.build_function_parameter_term(*parameter, tree))
            .collect();

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
        };

        self.terms.push(function)
    }

    /// Return one function type term from a type-space constructor declaration.
    pub(in crate::check) fn build_constructor_type_declaration_term(
        &mut self,
        declaration: &dir::ConstructorTypeDeclaration,
        return_type: Option<VariableId>,
        tree: &dir::Tree,
    ) -> TermId<FunctionTerm> {
        // collect constructor generic parameters
        let generic_parameters = declaration
            .generic_parameters
            .iter()
            .filter_map(|parameter| self.intern_generic_parameter_variable(*parameter, tree))
            .collect();

        // collect constructor runtime parameters
        let parameters = declaration
            .parameters
            .iter()
            .filter_map(|parameter| self.build_function_parameter_term(*parameter, tree))
            .collect();

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
        };

        self.terms.push(function)
    }

    /// Walk one function body inside a function flow frame.
    pub(in crate::check) fn walk_function_body(
        &mut self,
        tree: &dir::Tree,
        symbol: dir::GlobalSymbolId,
        signature: &dir::FunctionSignature,
        body: dir::LocalNodeId<dir::Expression>,
        return_type: VariableId,
        receiver: Option<ReceiverCapture>,
    ) {
        let mut body_return_type = return_type;
        let mut yield_type = None;
        let mut resume_type = None;

        // build async result channel
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let source = body.into_global_any(tree.module_id);
            let origin = ConstraintOrigin::Node(source);
            let completed =
                self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
            let Ok(symbol) = self.language_symbol(tree.module_id, dir::LanguageItem::Promise)
            else {
                return;
            };
            let argument = GenericArgument::Type(completed.into());
            let promised = TypeTerm::Reference {
                source: Some(source),
                symbol,
                arguments: vec![argument].into(),
            };
            let promised_variable =
                self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
            self.define_type(tree.module_id, promised_variable, promised);

            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                promised_variable,
                return_type,
            );
            body_return_type = completed;
        }

        // build generator channels
        if signature.is_generator {
            let source = body.into_global_any(tree.module_id);
            let origin = ConstraintOrigin::Node(source);
            let yielded =
                self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
            let completed =
                self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
            let resumed =
                self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };

            if let Ok(symbol) = self.language_symbol(tree.module_id, item) {
                let yielded = GenericArgument::Type(yielded.into());
                let completed = GenericArgument::Type(completed.into());
                let resumed = GenericArgument::Type(resumed.into());
                let generated = TypeTerm::Reference {
                    source: Some(source),
                    symbol,
                    arguments: vec![yielded, completed, resumed].into(),
                };
                let generated_variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                self.define_type(tree.module_id, generated_variable, generated);

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    generated_variable,
                    return_type,
                );
            } else {
                return;
            }

            body_return_type = completed;
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
        if !Self::function_is_constructor(signature) && self.expression_can_fall_through(tree, body)
        {
            self.constrain_function_fallthrough_return(tree, body);
        }

        self.leave_function_frame(tree.module_id);
    }

    /// Return whether one signature is a constructor body.
    fn function_is_constructor(signature: &dir::FunctionSignature) -> bool {
        matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        )
    }

    /// Return one runtime function parameter term.
    fn build_function_parameter_term(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<FunctionParameter> {
        let parameter = tree.get(id);
        let ty = self.intern_parameter_type_variable(id, tree)?;

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
            ty: ty.into(),
            is_optional,
            is_rest,
        };

        Some(parameter)
    }

    /// Return the variable for one generic parameter.
    fn intern_generic_parameter_variable(
        &mut self,
        id: dir::LocalNodeId<dir::GenericParameter>,
        tree: &dir::Tree,
    ) -> Option<VariableId> {
        let parameter = tree.get(id);

        self.record_generic_parameter_slot(tree.module_id, id, parameter)
    }

    /// Return the type variable for one runtime parameter.
    ///
    /// Missing annotations use the parameter node variable so contextual lambdas can still receive
    /// an expected type.
    pub(in crate::check) fn intern_parameter_type_variable(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<VariableId> {
        let parameter = tree.get(id);
        if matches!(parameter, dir::Parameter::Error) {
            return None;
        }

        // contextual lambdas can receive a parameter type later
        let Some(declared_type) = parameter.declared_type() else {
            return Some(self.intern_local_type_variable(tree.module_id, id));
        };

        Some(self.intern_local_type_variable(tree.module_id, declared_type))
    }
}
