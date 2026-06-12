use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    GenericInductionPosition, GenericTemplateId, Origin, ReceiverBinding, Relation, WalkState,
};

impl<'check, 'state> WalkState<'check, 'state> {
    /// Return one function type from a function signature.
    ///
    /// Example:
    /// ```ds
    /// function run<T>(value: T): T { value }
    /// ```
    pub(in crate::check) fn function_signature_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        template: Option<GenericTemplateId>,
        receiver_type: Option<dir::GlobalTypeId>,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let generic_parameters = self.signature_generic_parameters(source, template)?;

        let this_parameter = if let Some(parameter) = signature.this_parameter {
            self.parameter_type(parameter)?.or(receiver_type)
        } else {
            receiver_type
        };

        // collect runtime parameters
        let mut parameters = Vec::new();
        for parameter in &signature.parameters {
            if let Some(parameter) = self.function_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }

        let function = dir::FunctionType {
            asynchrony: signature.asynchrony,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
        };

        self.push_type(dir::Type::Function(function), source)
    }

    /// Return one function type from a type-space function declaration.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    pub(in crate::check) fn function_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declaration: &dir::FunctionTypeExpression,
        template: Option<GenericTemplateId>,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let generic_parameters = self.signature_generic_parameters(source, template)?;

        let this_parameter = if let Some(parameter) = declaration.this_parameter {
            self.parameter_type(parameter)?
        } else {
            None
        };

        // collect runtime parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.function_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }

        let function = dir::FunctionType {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
        };

        self.push_type(dir::Type::Function(function), source)
    }

    /// Return one function type from a type-space constructor declaration.
    ///
    /// Example:
    /// ```ds
    /// new (value: T) => Box<T>
    /// ```
    pub(in crate::check) fn constructor_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declaration: &dir::ConstructorType,
        template: Option<GenericTemplateId>,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let generic_parameters = self.signature_generic_parameters(source, template)?;

        // collect runtime parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.function_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }

        let function = dir::FunctionType {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
        };

        self.push_type(dir::Type::Function(function), source)
    }

    /// Return the parameter types captured by one callable signature.
    fn signature_generic_parameters(
        &mut self,
        source: dir::LocalNodeIdAny,
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(template) = template else {
            return Ok(Vec::new());
        };

        // write each declared parameter as its parameter type
        let parameters = self.check.generic_template_parameters(template);
        let mut types = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            types.push(self.push_type(dir::Type::Parameter(parameter), source)?);
        }

        Ok(types)
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
        result: dir::GlobalTypeId,
        receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<()> {
        let source = body.into_any();
        let origin = Origin::Node(body.into_global_any(self.module));
        let mut return_target = result;
        let mut yield_target = None;
        let mut resume_target = None;

        // open the async result channel
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let completed = self.open_type(source)?;
            let promised =
                self.language_type_reference(source, dir::LanguageItem::Promise, vec![completed])?;
            self.relate_type(origin, Relation::Assignable, promised, result);

            return_target = completed;
        }

        // open the generator channels
        if signature.is_generator {
            let yielded = self.open_type(source)?;
            let completed = self.open_type(source)?;
            let resumed = self.open_type(source)?;
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };
            let generated =
                self.language_type_reference(source, item, vec![yielded, completed, resumed])?;
            self.relate_type(origin, Relation::Assignable, generated, result);

            return_target = completed;
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

        // walk body and flow its completion value into the return
        self.walk_expression(body, self.tree.get(body))?;
        if !Self::is_constructor_signature(signature) && self.expression_can_complete_normally(body)
        {
            let completion = self.node_type(body)?;
            self.relate_type(origin, Relation::Assignable, completion, return_target);
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

    /// Return one runtime function parameter type.
    ///
    /// Example:
    /// ```ds
    /// (value?: T, ...rest: U[])
    /// ```
    fn function_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<dir::FunctionParameterType>> {
        let parameter = self.tree.get(id);
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );
        let is_optional = parameter.is_optional();
        let is_comptime = parameter.is_comptime();
        let Some(ty) = self.parameter_type(id)? else {
            return Ok(None);
        };

        // comptime parameters supply their generic parameter statically
        let static_parameter = if is_comptime {
            let source = id.into_any();
            let Some(symbol) = self.check.module(self.module).declaration_symbol(source) else {
                return Ok(None);
            };

            self.check.generics.parameter_by_symbol(symbol)
        } else {
            None
        };

        Ok(Some(dir::FunctionParameterType {
            ty,
            static_parameter,
            is_optional,
            is_rest,
        }))
    }

    /// Return one runtime parameter type.
    ///
    /// Missing annotations use the parameter node variable so contextual
    /// lambdas can still receive an expected type.
    ///
    /// Example:
    /// ```ds
    /// (value: T)
    /// ```
    pub(in crate::check) fn parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let declared_type = match self.tree.get(id) {
            dir::Parameter::Error => return Ok(None),
            parameter => parameter.declared_type(),
        };
        let Some(declared_type) = declared_type else {
            return Ok(Some(self.node_type(id)?));
        };

        let is_optional = self.tree.get(id).is_optional();
        let spelled = self.walk_type_expression(declared_type)?;
        let spelled = self.induce_constraint_type(
            id.into_any(),
            spelled,
            GenericInductionPosition::Parameter,
        )?;
        let spelled = if is_optional {
            self.optional_value_type(spelled, id.into_any())?
        } else {
            spelled
        };

        Ok(Some(spelled))
    }
}
