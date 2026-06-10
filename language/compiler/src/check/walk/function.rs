use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    FunctionParameter, FunctionTerm, GenericArgument, GenericInductionPosition, GenericParameterId,
    GenericTemplateId, Origin, ReceiverBinding, SubstitutionSet, TermId, TypeOperand, TypeRelation,
    TypeTerm, WalkState,
};

impl<'check, 'state> WalkState<'check, 'state> {
    /// Return one function type term from a function signature.
    ///
    /// Example:
    /// ```ds
    /// function run<T>(value: T): T { value }
    /// ```
    pub(in crate::check) fn function_signature_term(
        &mut self,
        signature: &dir::FunctionSignature,
        template: Option<GenericTemplateId>,
        receiver_type: Option<TypeOperand>,
        return_type: Option<TypeOperand>,
    ) -> CompilerResult<TermId<FunctionTerm>> {
        let generic_parameters = self.signature_generic_parameters(template);

        let this_parameter = if let Some(parameter) = signature.this_parameter {
            self.parameter_type(parameter)?.or(receiver_type)
        } else {
            receiver_type
        };

        let mut parameters = Vec::new();

        // collect runtime parameters
        for parameter in &signature.parameters {
            if let Some(parameter) = self.function_parameter_term(*parameter)? {
                parameters.push(parameter);
            }
        }

        let function = FunctionTerm {
            asynchrony: signature.asynchrony,
            generic_parameters: generic_parameters.into(),
            this_parameter,
            parameters: parameters.into(),
            return_type,
            is_generator: signature.is_generator,
        };
        let function = if let Some(receiver_type) = receiver_type {
            let substitution = SubstitutionSet::with_receiver(receiver_type);

            function.substitute(self.module, &substitution, self.check)?
        } else {
            function
        };

        Ok(self.check.inference.push_term(function))
    }

    /// Return one function type term from a type-space function declaration.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    pub(in crate::check) fn function_type_term(
        &mut self,
        declaration: &dir::FunctionTypeExpression,
        template: Option<GenericTemplateId>,
        return_type: Option<TypeOperand>,
    ) -> CompilerResult<TermId<FunctionTerm>> {
        let generic_parameters = self.signature_generic_parameters(template);

        let this_parameter = if let Some(parameter) = declaration.this_parameter {
            self.parameter_type(parameter)?
        } else {
            None
        };

        let mut parameters = Vec::new();

        // collect runtime parameters
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.function_parameter_term(*parameter)? {
                parameters.push(parameter);
            }
        }

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: generic_parameters.into(),
            this_parameter,
            parameters: parameters.into(),
            return_type,
            is_generator: false,
        };

        Ok(self.check.inference.push_term(function))
    }

    /// Return one function type term from a type-space constructor declaration.
    ///
    /// Example:
    /// ```ds
    /// new (value: T) => Box<T>
    /// ```
    pub(in crate::check) fn constructor_type_term(
        &mut self,
        declaration: &dir::ConstructorType,
        template: Option<GenericTemplateId>,
        return_type: Option<TypeOperand>,
    ) -> CompilerResult<TermId<FunctionTerm>> {
        let generic_parameters = self.signature_generic_parameters(template);

        let mut parameters = Vec::new();

        // collect runtime parameters
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.function_parameter_term(*parameter)? {
                parameters.push(parameter);
            }
        }

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: generic_parameters.into(),
            this_parameter: None,
            parameters: parameters.into(),
            return_type,
            is_generator: false,
        };

        Ok(self.check.inference.push_term(function))
    }

    /// Return generic parameters captured by one callable signature.
    fn signature_generic_parameters(
        &self,
        template: Option<GenericTemplateId>,
    ) -> SmallVec<[GenericParameterId; 2]> {
        let mut parameters = SmallVec::new();

        // collect enclosing templates before nested templates
        if let Some(template) = template {
            self.collect_signature_generic_parameters(template, &mut parameters);
        }

        parameters
    }

    /// Append captured generic parameters from one template.
    fn collect_signature_generic_parameters(
        &self,
        template: GenericTemplateId,
        parameters: &mut SmallVec<[GenericParameterId; 2]>,
    ) {
        let Some(generic_template) = self.check.inference.generic_template(template) else {
            return;
        };

        // collect parent parameters first
        if let Some(parent) = generic_template.parent {
            self.collect_signature_generic_parameters(parent, parameters);
        }

        // collect local parameters in declaration order
        for parameter in &generic_template.parameters {
            parameters.push(*parameter);
        }
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
        symbol: dir::GlobalSymbolId,
        signature: &dir::FunctionSignature,
        body: dir::LocalNodeId<dir::Expression>,
        result: TypeOperand,
        receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<()> {
        let mut return_target = result;
        let mut yield_target = None;
        let mut resume_target = None;

        // allocate async result channel
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let source = body.into_global_any(self.module);
            let origin = Origin::Node(source);
            let completed = self.check.push_type_variable(self.module, origin);
            let symbol = self.check.language_symbol(dir::LanguageItem::Promise);
            let argument = GenericArgument::Type(completed.into());
            let promised = TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments: vec![argument].into(),
            };
            let promised = self.check.inference.push_term(promised);
            let condition = self.active_static_guard();
            self.check.constrain_type(
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
            let source = body.into_global_any(self.module);
            let origin = Origin::Node(source);
            let yielded = self.check.push_type_variable(self.module, origin);
            let completed = self.check.push_type_variable(self.module, origin);
            let resumed = self.check.push_type_variable(self.module, origin);
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };

            let symbol = self.check.language_symbol(item);
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
            self.check.constrain_type(
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
            self.mark_bindings_assigned(parameter.into_any());
        }
        for parameter in &signature.parameters {
            self.mark_bindings_assigned(parameter.into_any());
        }

        // walk body and constrain implicit completion
        self.walk_expression(body, self.tree.get(body))?;
        if !Self::is_constructor_signature(signature) && self.expression_can_complete_normally(body)
        {
            self.constrain_function_completion_return(body)?;
        }

        self.leave_function_frame()?;

        Ok(())
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
    fn function_parameter_term(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<FunctionParameter>> {
        let parameter = self.tree.get(id);
        let Some(ty) = self.parameter_type(id)? else {
            return Ok(None);
        };

        // rest parameters are variadic
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );

        let parameter = FunctionParameter {
            ty,
            static_parameter: if parameter.is_comptime() {
                let source = id.into_any();
                let Some(symbol) = self.check.module(self.module).declaration_symbol(source) else {
                    return Ok(None);
                };
                self.check.inference.generic_parameter_by_symbol(symbol)
            } else {
                None
            },
            is_inferred: parameter.declared_type().is_none(),
            is_optional: parameter.is_optional(),
            is_rest,
        };

        Ok(Some(parameter))
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
    ) -> CompilerResult<Option<TypeOperand>> {
        let declared_type = match self.tree.get(id) {
            dir::Parameter::Error => return Ok(None),
            parameter => parameter.declared_type(),
        };
        let Some(declared_type) = declared_type else {
            let operand = self.node_type_operand(id)?;

            return Ok(Some(operand));
        };

        let parameter = self.tree.get(id);
        let source = id.into_global_any(self.module);
        let operand = self.node_type_operand(declared_type)?;
        let condition = self.active_static_guard();
        let operand = self.induce_constraint_type_operand(
            source,
            operand,
            GenericInductionPosition::Parameter,
            condition,
        )?;
        let operand = if parameter.is_optional() {
            self.optional_value_type(operand)
        } else {
            operand
        };

        Ok(Some(operand))
    }
}
