use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Expectation, FlowBranch, GenericInductionDeclaration, GenericInductionPosition,
    GenericTemplateId, Origin, ReceiverBinding, Relation, ValueUse, WalkState, Widening,
};

impl<'check, 'state> WalkState<'check, 'state> {
    /// Walk one function signature and return its type.
    ///
    /// Example:
    /// ```ds
    /// function run<T>(value: T): T { value }
    /// ```
    pub(in crate::check) fn walk_function_signature_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        template: Option<GenericTemplateId>,
        owner: Option<GenericInductionDeclaration>,
        receiver_type: Option<dir::GlobalTypeId>,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let this_parameter = if let Some(parameter) = signature.this_parameter {
            receiver_type.or(self.walk_parameter_type(parameter)?)
        } else {
            receiver_type
        };

        // collect runtime parameters
        let mut parameters = Vec::new();
        for parameter in &signature.parameters {
            if let Some(parameter) = self.walk_function_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }
        let template =
            self.signature_template(template, owner, this_parameter, &parameters, return_type)?;

        let function = dir::FunctionSignatureType {
            asynchrony: signature.asynchrony,
            template,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
        };

        self.push_type(dir::Type::FunctionSignature(function), source)
    }

    /// Return the template owned by one callable signature.
    fn signature_template(
        &mut self,
        template: Option<GenericTemplateId>,
        owner: Option<GenericInductionDeclaration>,
        this_parameter: Option<dir::GlobalTypeId>,
        parameters: &[dir::FunctionParameterType],
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if template.is_some() {
            return Ok(template);
        }
        let Some(owner) = owner else {
            return Ok(None);
        };

        // open an owner template only when this signature contains induced holes
        if !self.signature_contains_induced_parameter(this_parameter, parameters, return_type)? {
            return Ok(None);
        }

        self.check
            .open_generic_template(owner.declaration, owner.parent, owner.symbol)
            .map(Some)
    }

    /// Return whether one signature contains an induced generic hole.
    fn signature_contains_induced_parameter(
        &mut self,
        this_parameter: Option<dir::GlobalTypeId>,
        parameters: &[dir::FunctionParameterType],
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        let mut types = Vec::new();
        types.extend(this_parameter);
        types.extend(parameters.iter().map(|parameter| parameter.ty));
        types.extend(return_type);

        for ty in types {
            for variable in self.check.type_variables(ty)? {
                let representative = self.check.solver.representative(variable)?;
                if self.check.generics.induction(representative).is_some() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Walk one function type expression.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    pub(in crate::check) fn walk_function_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declaration: &dir::FunctionTypeExpression,
        parent: Option<GenericTemplateId>,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let template = self.walk_signature_template(
            source,
            parent,
            declaration.declares_generic_template(&self.tree),
            &declaration.generic_parameters,
            &declaration.where_clauses,
        )?;
        let return_type = match (return_type, declaration.return_type) {
            (Some(return_type), _) => Some(return_type),
            (None, Some(return_type)) => {
                Some(self.walk_return_type_expression(source, return_type, false)?)
            }
            (None, None) => None,
        };

        let this_parameter = if let Some(parameter) = declaration.this_parameter {
            self.walk_parameter_type(parameter)?
        } else {
            None
        };

        // collect signature parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.walk_signature_parameter_type(template, *parameter)? {
                parameters.push(parameter);
            }
        }

        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
        };
        let signature = self.push_type(dir::Type::FunctionSignature(function), source)?;

        self.push_function_value_type(source, signature)
    }

    /// Walk one constructor type expression.
    ///
    /// Example:
    /// ```ds
    /// new (value: T) => Box<T>
    /// ```
    pub(in crate::check) fn walk_constructor_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declaration: &dir::ConstructorType,
        parent: Option<GenericTemplateId>,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let template = self.walk_signature_template(
            source,
            parent,
            declaration.declares_generic_template(&self.tree),
            &declaration.generic_parameters,
            &declaration.where_clauses,
        )?;
        let return_type = match (return_type, declaration.return_type) {
            (Some(return_type), _) => Some(return_type),
            (None, Some(return_type)) => {
                Some(self.walk_return_type_expression(source, return_type, false)?)
            }
            (None, None) => None,
        };

        // collect signature parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.walk_signature_parameter_type(template, *parameter)? {
                parameters.push(parameter);
            }
        }

        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
        };

        self.push_type(dir::Type::FunctionSignature(function), source)
    }

    /// Return one fat callable value type for a function signature.
    pub(in crate::check) fn push_function_value_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let environment = self.push_type(dir::Type::Unknown, source)?;
        let function = dir::FunctionType {
            signature,
            environment,
        };

        self.push_type(dir::Type::Function(function), source)
    }

    /// Return the template owned by one callable type header.
    fn walk_signature_template(
        &mut self,
        source: dir::LocalNodeIdAny,
        parent: Option<GenericTemplateId>,
        declares_template: bool,
        generic_parameters: &[dir::LocalNodeId<dir::GenericParameter>],
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if !declares_template {
            return Ok(None);
        }

        let source = source.into_global(self.module);
        let template = if generic_parameters.is_empty() {
            self.check.open_generic_template(source, parent, None)?
        } else {
            let Some(template) =
                self.walk_generic_template(source, parent, None, generic_parameters)?
            else {
                return Ok(None);
            };

            template
        };

        // apply where clauses after all header parameters exist
        for where_clause in where_clauses {
            self.walk_where_clause(*where_clause)?;
        }

        Ok(Some(template))
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
    ) -> CompilerResult<FlowBranch> {
        let source = body.into_any();
        let origin = Origin::Node(body.into_global_any(self.module));
        let mut return_target = result;
        let mut yield_target = None;
        let mut resume_target = None;

        // open the async completion type
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let completed = self.open_variable_type(source, Widening::Preserve)?;
            let promised =
                self.language_type_reference(source, dir::LanguageItem::Promise, vec![completed])?;
            self.relate_type(origin, Relation::Assignable, promised, result);

            return_target = completed;
        }

        // open the generator yielded, completed, and resumed types
        if signature.is_generator {
            let yielded = self.open_variable_type(source, Widening::Preserve)?;
            let completed = self.open_variable_type(source, Widening::Preserve)?;
            let resumed = self.open_variable_type(source, Widening::Preserve)?;
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
        let expectation = (!Self::is_constructor_signature(signature)
            && self.expression_can_complete_normally(body))
        .then(|| Expectation::assignable(return_target, origin, ValueUse::Output));
        self.walk_expression(body, self.tree.get(body), expectation.as_ref())?;

        self.leave_function_frame()
    }

    /// Return whether one signature is a constructor body.
    fn is_constructor_signature(signature: &dir::FunctionSignature) -> bool {
        matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
        )
    }

    /// Walk one runtime function parameter and return its signature slot.
    ///
    /// Example:
    /// ```ds
    /// (value?: T, ...rest: U[])
    /// ```
    fn walk_function_parameter_type(
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
        let Some(ty) = self.walk_parameter_type(id)? else {
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

    /// Walk one callable type parameter and return its signature slot.
    fn walk_signature_parameter_type(
        &mut self,
        template: Option<GenericTemplateId>,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<dir::FunctionParameterType>> {
        let parameter = self.tree.get(id);
        if parameter.declared_type().is_none() {
            self.check
                .report_missing_type_annotation(self.module, id.into_any());
        }
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );
        let is_optional = parameter.is_optional();
        let is_comptime = parameter.is_comptime();
        let Some(written) = self.walk_parameter_type(id)? else {
            return Ok(None);
        };

        // comptime parameters are static generic parameters at call sites
        let static_parameter = if is_comptime {
            let source = id.into_any();
            let symbol = self.check.module(self.module).declaration_symbol(source);
            let default = match parameter {
                dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => {
                    *default
                }
                _ => None,
            };

            self.induce_comptime_parameter(
                template,
                source,
                symbol,
                Some(written),
                default,
                is_rest,
            )?
        } else {
            None
        };

        Ok(Some(dir::FunctionParameterType {
            ty: written,
            static_parameter,
            is_optional,
            is_rest,
        }))
    }

    /// Walk one parameter annotation and return its type.
    ///
    /// Missing annotations use the parameter node variable so contextual
    /// lambdas can still receive an expected type.
    ///
    /// Example:
    /// ```ds
    /// (value: T)
    /// ```
    pub(in crate::check) fn walk_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let node = id.into_global_any(self.module);
        if let Some(ty) = self.check.node_type_maybe(node) {
            return Ok(Some(ty));
        }

        let declared_type = match self.tree.get(id) {
            dir::Parameter::Error => return Ok(None),
            parameter => parameter.declared_type(),
        };
        let Some(declared_type) = declared_type else {
            let ty = self.open_variable_type(id.into_any(), Widening::Preserve)?;
            self.write_node_type(id, ty)?;

            return Ok(Some(ty));
        };

        let is_optional = self.tree.get(id).is_optional();
        let written = self.walk_type_expression(declared_type)?;
        let written = self.induce_constraint_type(
            id.into_any(),
            written,
            GenericInductionPosition::Parameter,
        )?;
        let written = if is_optional {
            self.optional_value_type(written, id.into_any())?
        } else {
            written
        };
        self.write_node_type(id, written)?;

        Ok(Some(written))
    }
}
