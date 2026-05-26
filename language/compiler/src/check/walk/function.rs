use destack_dir as dir;

use crate::check::{
    ArgumentTerm, CheckModuleState, ConstraintOrigin, FunctionParameterTerm, FunctionTerm,
    ReceiverCapture, TypeRelation, TypeTerm, VariableId, VariableKind,
};

impl CheckModuleState {
    /// Return one function type term from a function signature.
    pub(in crate::check) fn function_signature_term(
        &mut self,
        signature: &dir::FunctionSignature,
        return_type: Option<VariableId>,
        tree: &dir::Tree,
    ) -> FunctionTerm {
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
            .filter_map(|parameter| self.function_parameter_term(*parameter, tree))
            .collect();

        FunctionTerm {
            asynchrony: signature.asynchrony,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
        }
    }

    /// Return one function type term from a type-space function declaration.
    pub(in crate::check) fn function_type_declaration_term(
        &mut self,
        declaration: &dir::FunctionTypeDeclaration,
        return_type: Option<VariableId>,
        tree: &dir::Tree,
    ) -> FunctionTerm {
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
            .filter_map(|parameter| self.function_parameter_term(*parameter, tree))
            .collect();

        FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
        }
    }

    /// Return one function type term from a type-space constructor declaration.
    pub(in crate::check) fn constructor_type_declaration_term(
        &mut self,
        declaration: &dir::ConstructorTypeDeclaration,
        return_type: Option<VariableId>,
        tree: &dir::Tree,
    ) -> FunctionTerm {
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
            .filter_map(|parameter| self.function_parameter_term(*parameter, tree))
            .collect();

        FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
        }
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
            let source = body.into_global_any(self.input.module_id);
            let origin = ConstraintOrigin::Node(source);
            let completed = self.allocate_anonymous_variable(VariableKind::Type, origin);
            let Some(symbol) = self
                .input
                .environment
                .language
                .symbol(dir::LanguageItem::Promise)
            else {
                self.report_internal_error(
                    body.into_any(),
                    "missing language item: async.Promise".to_owned(),
                );

                return;
            };
            let promised = TypeTerm::Reference {
                source: Some(source),
                symbol,
                arguments: vec![ArgumentTerm::Type(completed)],
            };
            let promised = self.define_type(origin, promised);

            self.relate_type(origin, TypeRelation::Assignable, promised, return_type);
            body_return_type = completed;
        }

        // build generator channels
        if signature.is_generator {
            let source = body.into_global_any(self.input.module_id);
            let origin = ConstraintOrigin::Node(source);
            let yielded = self.allocate_anonymous_variable(VariableKind::Type, origin);
            let completed = self.allocate_anonymous_variable(VariableKind::Type, origin);
            let resumed = self.allocate_anonymous_variable(VariableKind::Type, origin);
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };

            if let Some(symbol) = self.input.environment.language.symbol(item) {
                let generated = TypeTerm::Reference {
                    source: Some(source),
                    symbol,
                    arguments: vec![
                        ArgumentTerm::Type(yielded),
                        ArgumentTerm::Type(completed),
                        ArgumentTerm::Type(resumed),
                    ],
                };
                let generated = self.define_type(origin, generated);

                self.relate_type(origin, TypeRelation::Assignable, generated, return_type);
            } else {
                self.report_internal_error(
                    body.into_any(),
                    format!("missing language item: {}", item.key()),
                );

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

        // walk body and relate implicit return
        self.walk_expression(tree, body, tree.get(body));
        if self.expression_can_fall_through(tree, body) {
            self.constrain_function_fallthrough_return(tree, body);
        }

        self.leave_function_frame();
    }

    /// Return one runtime function parameter term.
    fn function_parameter_term(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
    ) -> Option<FunctionParameterTerm> {
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

        Some(FunctionParameterTerm {
            ty,
            is_optional,
            is_rest,
        })
    }

    /// Return the variable for one generic parameter.
    fn intern_generic_parameter_variable(
        &mut self,
        id: dir::LocalNodeId<dir::GenericParameter>,
        tree: &dir::Tree,
    ) -> Option<VariableId> {
        let parameter = tree.get(id);

        match parameter {
            // <T>, <...T>
            dir::GenericParameter::Type { .. } | dir::GenericParameter::VariadicType { .. } => self
                .declaration_symbol(id.into_any())
                .map(|symbol| self.intern_symbol_type_variable(symbol)),
            // <comptime C: T>, <comptime ...C: T>
            dir::GenericParameter::Value { .. } | dir::GenericParameter::VariadicValue { .. } => {
                self.declaration_symbol(id.into_any())
                    .map(|symbol| self.intern_symbol_static_variable(symbol))
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => None,
        }
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
            return Some(self.intern_local_type_variable(id));
        };

        Some(self.intern_local_type_variable(declared_type))
    }
}
