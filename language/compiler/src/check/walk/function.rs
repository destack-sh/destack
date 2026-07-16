use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    BodyOwner, BodyPhase, BodyTarget, CauseKind, ExpectedType, FlowBranch, GeneratorTargets,
    GenericTemplateId, InducedParameterOwner, Origin, ReceiverBinding, Relation, ValueUse,
    VariableRole, WalkState, Widening,
};

/// Runtime parameter types produced by one function signature header.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionHeader {
    /// The generic template declared by the signature.
    pub(in crate::check) template: Option<GenericTemplateId>,
    /// The explicit `this` parameter type.
    pub(in crate::check) this_parameter: Option<dir::GlobalTypeId>,
    /// The runtime parameter types.
    pub(in crate::check) parameters: Vec<dir::FunctionParameterType>,
}

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
        header: FunctionHeader,
        owner: Option<InducedParameterOwner>,
        receiver_type: Option<dir::GlobalTypeId>,
        return_type: Option<dir::GlobalTypeId>,
        tracked: Vec<dir::TypeVariableId>,
        has_body: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let parameters = header.parameters;
        // elision reads the annotated receiver, which carries its borrow;
        //  rung 3 may synthesize a readonly receiver borrow
        let (return_type, synthesized_this) = self.apply_result_lifetime_elision(
            source,
            header.this_parameter,
            receiver_type,
            &parameters,
            return_type,
            tracked,
            has_body,
        )?;
        let this_parameter = header.this_parameter.or(synthesized_this).or(receiver_type);
        let template = self.signature_template(
            header.template,
            owner,
            this_parameter,
            &parameters,
            return_type,
        )?;

        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: signature.asynchrony,
            template,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
        };

        self.intern_signature(function)
    }

    /// Apply elided result lifetimes to the receiver or unique input borrow lifetime.
    pub(in crate::check) fn apply_result_lifetime_elision(
        &mut self,
        source: dir::LocalNodeIdAny,
        this_parameter: Option<dir::GlobalTypeId>,
        receiver_type: Option<dir::GlobalTypeId>,
        parameters: &[dir::FunctionParameterType],
        return_type: Option<dir::GlobalTypeId>,
        tracked: Vec<dir::TypeVariableId>,
        has_body: bool,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Option<dir::GlobalTypeId>)> {
        let Some(return_type) = return_type else {
            return Ok((None, None));
        };

        // the receiver's lifetime wins over value parameter lifetimes,
        //  de-duplicating aliases introduced by reused annotations
        let mut seen = FxIndexSet::default();
        let mut input_lifetimes = Vec::new();
        if let Some(this_parameter) = this_parameter {
            input_lifetimes.extend(self.induced_lifetime_types(this_parameter)?);
            input_lifetimes.retain(|(variable, _)| seen.insert(*variable));
        }
        if input_lifetimes.is_empty() {
            for parameter in parameters {
                input_lifetimes.extend(self.induced_lifetime_types(parameter.ty)?);
            }
            input_lifetimes.retain(|(variable, _)| seen.insert(*variable));
        }

        // synthesize a readonly receiver borrow for an elided result lifetime
        let mut synthesized_this = None;
        if input_lifetimes.is_empty()
            && this_parameter.is_none()
            && let Some(receiver) = receiver_type
        {
            let has_elided_result =
                !tracked.is_empty() || !self.induced_lifetime_types(return_type)?.is_empty();
            if has_elided_result {
                let lifetime = self.generated_receiver_borrow_lifetime(source)?;
                let access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                    dir::Access::Readonly,
                )))?;
                let form = self.intern_borrow(lifetime, access)?;
                let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
                    form,
                    value: receiver,
                }))?;
                synthesized_this = Some(borrowed);
                input_lifetimes.extend(self.induced_lifetime_types(borrowed)?);
                input_lifetimes.retain(|(variable, _)| seen.insert(*variable));
            }
        }

        let [(input_variable, input_lifetime)] = input_lifetimes.as_slice() else {
            // bodyless returns cannot infer their lifetimes from anywhere
            if !tracked.is_empty() && !has_body {
                self.check
                    .report_bodyless_lifetime_elided(self.module, source);
                for variable in tracked {
                    let error = self.intern_type(dir::Type::Error)?;
                    self.check.commit_solution(variable, error)?;
                }
            }
            // ambiguous inputs leave result lifetimes to body inference
            else if has_body {
                let return_lifetimes = self.induced_lifetime_types(return_type)?;
                for (variable, _) in return_lifetimes {
                    self.check.body_inferred_parameters.insert(variable);
                }
                for variable in tracked {
                    self.check.body_inferred_parameters.insert(variable);
                }
            }

            return Ok((Some(return_type), synthesized_this));
        };
        let input_lifetime = *input_lifetime;
        let mut return_lifetimes = self
            .induced_lifetime_types(return_type)?
            .into_iter()
            .map(|(variable, _)| variable)
            .collect::<Vec<_>>();
        return_lifetimes.extend(tracked);

        // tie each elided result lifetime to the receiver or input lifetime
        for return_variable in return_lifetimes {
            if return_variable != *input_variable {
                self.check
                    .commit_solution(return_variable, input_lifetime)?;
            }
        }

        Ok((Some(return_type), synthesized_this))
    }

    /// Return induced lifetime variables inside one type graph.
    fn induced_lifetime_types(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(dir::TypeVariableId, dir::GlobalTypeId)>> {
        let mut lifetimes = Vec::new();

        // collect open lifetime variables from the type graph
        for (variable, role) in self.check.induced_memory_variables(ty)? {
            let VariableRole::Memory {
                kind: dir::MemoryParameter::Lifetime,
                ..
            } = role
            else {
                continue;
            };

            let ty = self.check.variable_type(variable)?;
            lifetimes.push((variable, ty));
        }

        Ok(lifetimes)
    }

    /// Return the template owned by one callable signature.
    fn signature_template(
        &mut self,
        template: Option<GenericTemplateId>,
        owner: Option<InducedParameterOwner>,
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

        // open the enclosing template only when this signature has induced parameters
        if !self.signature_contains_induced_parameter(this_parameter, parameters, return_type)? {
            return Ok(None);
        }

        self.check
            .open_generic_template(owner.declaration, owner.parent, owner.symbol)
            .map(Some)
    }

    /// Return whether one signature contains an induced memory parameter.
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
            if !self.check.induced_memory_variables(ty)?.is_empty() {
                return Ok(true);
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
            declaration.declares_generic_scope(),
            &declaration.generic_parameters,
            &declaration.where_clauses,
        )?;
        let (return_type, tracked) = match (return_type, declaration.return_type) {
            (Some(return_type), _) => (Some(return_type), Vec::new()),
            (None, Some(return_type)) => {
                let (return_type, tracked) = self.walk_return_type_expression(return_type)?;

                (Some(return_type), tracked)
            }
            (None, None) => (None, Vec::new()),
        };

        let this_parameter = if let Some(parameter) = declaration.this_parameter {
            self.walk_parameter_type(parameter, false)?
        } else {
            None
        };

        // collect signature parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.walk_signature_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }

        let (return_type, _) = self.apply_result_lifetime_elision(
            source,
            this_parameter,
            None,
            &parameters,
            return_type,
            tracked,
            false,
        )?;
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
        };
        let signature = self.intern_signature(function)?;

        self.push_function_value_type(signature)
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
            declaration.declares_generic_scope(),
            &declaration.generic_parameters,
            &declaration.where_clauses,
        )?;
        let (return_type, tracked) = match (return_type, declaration.return_type) {
            (Some(return_type), _) => (Some(return_type), Vec::new()),
            (None, Some(return_type)) => {
                let (return_type, tracked) = self.walk_return_type_expression(return_type)?;

                (Some(return_type), tracked)
            }
            (None, None) => (None, Vec::new()),
        };

        // collect signature parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.walk_signature_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }

        let (return_type, _) = self.apply_result_lifetime_elision(
            source,
            None,
            None,
            &parameters,
            return_type,
            tracked,
            false,
        )?;
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
        };

        self.intern_signature(function)
    }

    /// Return one fat callable value type for a function signature.
    pub(in crate::check) fn push_function_value_type(
        &mut self,
        signature: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let environment = self.intern_type(dir::Type::Unknown)?;
        let function = dir::FunctionType {
            signature,
            environment,
        };

        self.intern_type(dir::Type::Function(function))
    }

    /// Return the template owned by one callable type header.
    fn walk_signature_template(
        &mut self,
        source: dir::LocalNodeIdAny,
        parent: Option<GenericTemplateId>,
        declares_scope: bool,
        generic_parameters: &[dir::LocalNodeId<dir::GenericParameter>],
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if !declares_scope {
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
            self.walk_where_clause(Some(template), *where_clause)?;
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
        let origin = Origin::Node(
            body.into_global_any(self.module),
            self.check.symbol_template(symbol)?,
        );
        let template = self.check.symbol_template(symbol)?;
        let _scope = self.enter_template_scope(template);
        let mut return_target = result;
        let mut yield_target = None;
        let mut resume_target = None;

        // open the async completion type
        if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
            let completed =
                self.open_type_hole(source, Widening::Preserve, VariableRole::Regular)?;
            let promised =
                self.language_type_reference(dir::LanguageItem::Promise, &[completed])?;
            self.relate_type(
                origin,
                CauseKind::Return { annotation: None },
                Relation::Assignable,
                promised,
                result,
            );

            return_target = completed;
        }

        // open the generator yielded, completed, and resumed types
        if signature.is_generator {
            let yielded = self.open_type_hole(source, Widening::Preserve, VariableRole::Regular)?;
            let completed =
                self.open_type_hole(source, Widening::Preserve, VariableRole::Regular)?;
            let resumed = self.open_type_hole(source, Widening::Preserve, VariableRole::Regular)?;
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };
            let generated = self.language_type_reference(item, &[yielded, completed, resumed])?;
            self.relate_type(
                origin,
                CauseKind::Return { annotation: None },
                Relation::Assignable,
                generated,
                result,
            );

            return_target = completed;
            yield_target = Some(yielded);
            resume_target = Some(resumed);
        }

        // choose the directive attached to this function value
        let capture_directive = match self.take_capture_directive() {
            Some(directive) => Some(directive),
            None => self.check.capture_directive_for_symbol(symbol)?,
        };

        // enter function flow
        self.enter_function_frame(
            symbol,
            return_target,
            yield_target,
            resume_target,
            signature.asynchrony,
            receiver,
            capture_directive,
        );

        // mark entry bindings as definitely assigned
        if let Some(parameter) = signature.this_parameter {
            self.mark_bindings_assigned(parameter.into_any());
        }
        for parameter in &signature.parameters {
            self.mark_bindings_assigned(parameter.into_any());
        }

        // walk the body structurally; the checker owns its judgments
        let body_node = match self.tree.get(body) {
            dir::Expression::Block(block) => {
                self.walk_block(*block, self.tree.get(*block))?;

                block.into_any()
            }
            _ => {
                self.walk_expression(body, self.tree.get(body))?;

                body.into_any()
            }
        };

        // record the body for the checker, in source order
        let ret = (!signature.is_constructor()).then_some(return_target);
        let generator = yield_target
            .zip(resume_target)
            .map(|(yielded, resumed)| GeneratorTargets { yielded, resumed });
        self.check.bodies.push(BodyOwner {
            phase: BodyPhase::Main,
            module: self.module,
            body: BodyTarget::Node(body_node),
            ret: ret.map(ExpectedType::Type),
            generator,
            ret_use: ValueUse::Output,
            binds: None,
            constructs: signature.is_constructor(),
        });

        Ok(self.leave_function_frame())
    }

    /// Return the signature slot for one walked runtime parameter.
    pub(in crate::check) fn function_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::FunctionParameterType>> {
        let parameter = self.tree.get(id);
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );
        // defaulted parameters may be omitted at the call site
        let is_optional = parameter.is_optional() || parameter.default_value().is_some();

        Ok(Some(dir::FunctionParameterType {
            ty,
            is_optional,
            is_rest,
        }))
    }

    /// Walk one callable type parameter and return its signature slot.
    fn walk_signature_parameter_type(
        &mut self,
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
        let Some(ty) = self.walk_parameter_type(id, false)? else {
            return Ok(None);
        };

        Ok(Some(dir::FunctionParameterType {
            ty,
            is_optional,
            is_rest,
        }))
    }

    /// Walk one parameter annotation and return its argument and binding types.
    ///
    /// Missing annotations use the parameter node variable so contextual
    /// lambdas can still receive an expected type.
    ///
    /// Example:
    /// ```ds
    /// (value?: T)
    /// ```
    pub(in crate::check) fn walk_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        represents_open_type: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let declared_type = match self.tree.get(id) {
            dir::Parameter::Error => return Ok(None),
            parameter => parameter.declared_type(),
        };
        let Some(declared_type) = declared_type else {
            let ty =
                self.open_type_hole(id.into_any(), Widening::Preserve, VariableRole::Regular)?;
            self.commit_node_type(id, ty)?;

            return Ok(Some(ty));
        };

        let is_optional = self.tree.get(id).is_optional();
        let ty = self.walk_type_expression(declared_type)?;
        let ty = if represents_open_type {
            self.check.storage_type(self.module, ty)?
        } else {
            ty
        };
        // optional parameters accept explicit undefined at call sites
        let ty = if is_optional {
            self.optional_value_type(ty)?
        } else {
            ty
        };
        self.commit_node_type(id, ty)?;

        Ok(Some(ty))
    }

    /// Return the canonical place type for one concrete space.
    pub(in crate::check) fn place_type(
        &mut self,
        space: dir::Space,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
            dir::Place::Space(space),
        )))
    }
}
