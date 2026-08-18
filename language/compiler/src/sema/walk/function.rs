use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    CauseKind, FlowBranch, FunctionBody, GeneratorTargets, GenericTemplateId,
    InducedParameterOwner, Origin, ReceiverBinding, Relation, VariableRole, WalkState, Widening,
};
use crate::{CompilerError, CompilerResult};

/// Runtime parameter types produced by one function signature header.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct FunctionHeader {
    /// The generic template declared by the signature.
    pub(in crate::sema) template: Option<GenericTemplateId>,
    /// The explicit `this` parameter type.
    pub(in crate::sema) this_parameter: Option<dir::GlobalTypeId>,
    /// The runtime parameter types.
    pub(in crate::sema) parameters: Vec<dir::FunctionParameterType>,
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Walk one function signature and return its type.
    ///
    /// Example:
    /// ```ds
    /// function run<T>(value: T): T { value }
    /// ```
    pub(in crate::sema) fn walk_function_signature_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        header: FunctionHeader,
        owner: Option<InducedParameterOwner>,
        receiver_type: Option<dir::GlobalTypeId>,
        return_type: Option<dir::GlobalTypeId>,
        tracked: Vec<dir::TypeVariableId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let parameters = header.parameters;

        // elide the result lifetime from the annotated or synthesized receiver
        let (return_type, synthesized_this) = self.apply_result_lifetime_elision(
            source,
            header.this_parameter,
            receiver_type,
            &parameters,
            return_type,
            tracked,
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
            is_construct: false,
        };

        self.intern_signature(function)
    }

    /// Apply elided result lifetimes: the receiver's lifetime, the unique
    /// input, the union of several inputs, or static storage.
    pub(in crate::sema) fn apply_result_lifetime_elision(
        &mut self,
        source: dir::LocalNodeIdAny,
        this_parameter: Option<dir::GlobalTypeId>,
        receiver_type: Option<dir::GlobalTypeId>,
        parameters: &[dir::FunctionParameterType],
        return_type: Option<dir::GlobalTypeId>,
        tracked: Vec<dir::TypeVariableId>,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Option<dir::GlobalTypeId>)> {
        let Some(mut return_type) = return_type else {
            return Ok((None, None));
        };

        // the receiver's lifetime wins over value parameter lifetimes,
        //  de-duplicating aliases introduced by reused annotations
        let mut seen = FxIndexSet::default();
        let mut input_lifetimes = Vec::new();
        if let Some(this_parameter) = this_parameter {
            input_lifetimes.extend(self.input_lifetime_types(this_parameter)?);
            input_lifetimes.retain(|(variable, ty)| seen.insert((*variable, *ty)));
        }
        if input_lifetimes.is_empty() {
            for parameter in parameters {
                input_lifetimes.extend(self.input_lifetime_types(parameter.ty)?);
            }
            input_lifetimes.retain(|(variable, ty)| seen.insert((*variable, *ty)));
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
                input_lifetimes.extend(self.input_lifetime_types(borrowed)?);
                input_lifetimes.retain(|(variable, ty)| seen.insert((*variable, *ty)));
            }
        }

        // collect the elided result lifetimes
        let mut return_lifetimes = self
            .induced_lifetime_types(return_type)?
            .into_iter()
            .map(|(variable, _)| variable)
            .collect::<Vec<_>>();
        return_lifetimes.extend(tracked);
        if return_lifetimes.is_empty() {
            return Ok((Some(return_type), synthesized_this));
        }

        // pick the result lifetime: the unique input, the union of
        //  several inputs, or static storage without borrowed inputs
        let input_variable = match input_lifetimes.as_slice() {
            [(variable, _)] => *variable,
            _ => None,
        };
        let input_lifetime = match input_lifetimes.as_slice() {
            [] => self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
                dir::Lifetime::Static,
            )))?,
            [(_, lifetime)] => *lifetime,
            _ => {
                let elements = input_lifetimes
                    .iter()
                    .map(|(_, lifetime)| *lifetime)
                    .collect::<Vec<_>>();

                self.check.normalized_union_type(elements)?
            }
        };

        // replace each elided result lifetime with the elected input
        for return_variable in return_lifetimes {
            if Some(return_variable) != input_variable {
                let return_lifetime = self.check.variable_type(return_variable)?;
                return_type = self.check.replace_type(
                    self.module,
                    return_type,
                    return_lifetime,
                    input_lifetime,
                )?;
            }
        }

        Ok((Some(return_type), synthesized_this))
    }

    /// Collect input lifetime terms: open elided holes and written lifetimes.
    fn input_lifetime_types(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(Option<dir::TypeVariableId>, dir::GlobalTypeId)>> {
        // collect open elided lifetime holes across the type graph
        let mut terms = Vec::new();
        for (variable, lifetime) in self.induced_lifetime_types(ty)? {
            terms.push((Some(variable), lifetime));
        }

        // collect written lifetime terms whole, without walking inside
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let node = self.check.ty(id)?;

            // follow solved variables toward their lifetime terms
            if let dir::Type::Variable(variable) = node {
                if let Some(solution) = self.check.infer.solution(variable)? {
                    pending.push(solution);
                }

                continue;
            }

            // collect closed lifetime terms and walk everything else
            if self.check.is_lifetime_term(id)? {
                terms.push((None, id));
            } else {
                self.check
                    .for_each_type_child(id.module_id, &node, |child| pending.push(child))?;
            }
        }

        Ok(terms)
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
            .open_generic_template(owner.declaration)
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
    pub(in crate::sema) fn walk_function_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declaration: &dir::FunctionTypeExpression,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let template = self.walk_signature_template(
            source,
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
            self.walk_parameter_type(parameter)?
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
        )?;

        // register signature positions whose elided borrows induce lifetime parameters
        let owner = InducedParameterOwner::new(source.into_global(self.module), None, None);
        for parameter in &parameters {
            self.push_induced_parameter_site(owner, parameter.ty);
        }
        if let Some(this_parameter) = this_parameter {
            self.push_induced_parameter_site(owner, this_parameter);
        }
        if let Some(return_type) = return_type {
            self.push_induced_parameter_site(owner, return_type);
        }
        let template = self.induced_owner_template(owner, template)?;

        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
            is_construct: false,
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
    pub(in crate::sema) fn walk_constructor_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declaration: &dir::ConstructorType,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let template = self.walk_signature_template(
            source,
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
        )?;
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
            is_construct: true,
        };

        self.intern_signature(function)
    }

    /// Return one fat callable value type for a function signature.
    pub(in crate::sema) fn push_function_value_type(
        &mut self,
        signature: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let function = dir::FunctionType {
            signature,
            multiplicity: dir::Multiplicity::Repeatable,
        };

        self.intern_type(dir::Type::Function(function))
    }

    /// Return the template owned by one callable type header.
    fn walk_signature_template(
        &mut self,
        source: dir::LocalNodeIdAny,
        declares_scope: bool,
        generic_parameters: &[dir::LocalNodeId<dir::GenericParameter>],
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if !declares_scope {
            return Ok(None);
        }

        let source = source.into_global(self.module);
        let template = if generic_parameters.is_empty() {
            self.check.open_generic_template(source)?
        } else {
            let Some(template) = self.walk_generic_template(source, generic_parameters)? else {
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
    pub(in crate::sema) fn walk_function_body(
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
            // infer unannotated async functions as promises
            if signature.return_type.is_none() {
                let completed =
                    self.open_type_hole(source, Widening::Never, VariableRole::Return)?;
                let promised =
                    self.language_type_reference(dir::LanguageItem::Promise, &[completed])?;
                let Some(variable) = self.check.root_variable(result)? else {
                    return Err(CompilerError::Internal {
                        message: "inferred async return is not an inference variable".into(),
                    });
                };
                self.check.commit_solution(variable, promised)?;
                return_target = completed;
            }
            // retain the declared Promise, Task, or transparent owner
            else if let Some(completed) = self.check.async_completion_type(result)? {
                return_target = completed;
            }
            // reject every other declared async result through the established relation
            else {
                let completed =
                    self.intern_operation(dir::TypeOperation::Awaited(dir::UnaryType {
                        target: result,
                    }))?;
                let promised =
                    self.language_type_reference(dir::LanguageItem::Promise, &[completed])?;
                self.relate_type(
                    origin,
                    CauseKind::Return { annotation: None },
                    Relation::Assignable,
                    promised,
                    result,
                )?;
            }
        }

        // open the generator yielded, completed, and resumed types
        if signature.is_generator {
            let yielded = self.open_type_hole(source, Widening::Never, VariableRole::Regular)?;
            let completed = self.open_type_hole(source, Widening::Never, VariableRole::Return)?;
            let resumed = self.open_type_hole(source, Widening::Never, VariableRole::Regular)?;
            let item = match signature.asynchrony {
                // function* f() {}
                dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                // async function* f() {}
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
            };
            let generated = self.language_type_reference(item, &[yielded, completed, resumed])?;
            if signature.return_type.is_none() {
                let Some(variable) = self.check.root_variable(result)? else {
                    return Err(CompilerError::Internal {
                        message: "inferred generator return is not an inference variable".into(),
                    });
                };
                self.check.commit_solution(variable, generated)?;
            } else {
                self.relate_type(
                    origin,
                    CauseKind::Return { annotation: None },
                    Relation::Assignable,
                    generated,
                    result,
                )?;
            }

            return_target = completed;
            yield_target = Some(yielded);
            resume_target = Some(resumed);
        }

        // collect the entry bindings the body assigns on entry
        let mut entries = SmallVec::<[dir::LocalNodeIdAny; 4]>::new();
        if let Some(parameter) = signature.this_parameter {
            entries.push(parameter.into_any());
        }
        for parameter in &signature.parameters {
            entries.push(parameter.into_any());
        }

        // enter the body node only: the check traversal owns its interior
        let body_site = match self.tree.get(body) {
            dir::Expression::Block(block) => self.enter_node(*block)?,
            _ => self.enter_node(body)?,
        };

        // record the body under its declaration identity
        let return_type = (!signature.is_constructor()).then_some(return_target);
        let generator =
            yield_target
                .zip(resume_target)
                .map(|(yielded, resumed)| GeneratorTargets {
                    asynchrony: signature.asynchrony,
                    yielded,
                    resumed,
                });
        // bind constructor initialization to its exact declaration
        let initializes = if signature.is_constructor() {
            let owner = receiver
                .as_ref()
                .and_then(|receiver| receiver.receiver.declaration)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("constructor {symbol:?} has no declaring receiver"),
                })?;

            Some(owner)
        } else {
            None
        };
        let body = FunctionBody {
            symbol,
            site: body_site,
            return_type,
            generator,
            initializes,
            asynchrony: signature.asynchrony,
            receiver,
            entries,
        };
        if self.check.functions.insert(symbol, body).is_some() {
            return Err(CompilerError::Internal {
                message: format!("function {symbol:?} has multiple checked bodies"),
            });
        }

        Ok(FlowBranch::default())
    }

    /// Return the signature slot for one walked runtime parameter.
    pub(in crate::sema) fn function_parameter_type(
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
            name: parameter.name(),
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
        let name = parameter.name();
        let Some(ty) = self.walk_parameter_type(id)? else {
            return Ok(None);
        };

        Ok(Some(dir::FunctionParameterType {
            name,
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
    pub(in crate::sema) fn walk_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let declared_type = match self.tree.get(id) {
            dir::Parameter::Error => return Ok(None),
            parameter => parameter.declared_type(),
        };
        let ty = if let Some(declared_type) = declared_type {
            let is_optional = self.tree.get(id).is_optional();
            let ty = self.walk_type_expression(declared_type)?;

            // optional parameters accept explicit undefined at call sites
            if is_optional {
                self.optional_value_type(ty)?
            } else {
                ty
            }
        } else {
            self.open_type_hole(id.into_any(), Widening::Never, VariableRole::Parameter)?
        };
        self.commit_node_type(id, ty)?;

        // strip undefined from a defaulted parameter's body binding
        let binding = match self.tree.get(id).default_value() {
            Some(_) => {
                let origin = Origin::Node(
                    id.into_global_any(self.module),
                    self.flow().template_scope(),
                );

                self.defaulted_value_type(origin, ty)?
            }
            None => ty,
        };

        // bind named parameters through the same path as their node type
        if let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        {
            self.bind_symbol_type(symbol, binding)?;
        }

        Ok(Some(ty))
    }
}
