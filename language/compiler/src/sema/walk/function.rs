use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    CauseKind, CoroutineBody, CoroutineForm, ElisionSite, FunctionBody, GeneratorTargets,
    GenericTemplateId, InducedParameterOwner, Origin, ReceiverBinding, Relation, VariableKind,
    WalkState,
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
        tracked: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // elide the result region from the walked header
        let parameters = header.parameters;

        // elide the result components from the annotated or synthesized receiver
        let (return_type, synthesized_this) = self.apply_result_elision(
            source,
            header.this_parameter,
            receiver_type,
            &parameters,
            return_type,
            tracked,
        )?;
        let this_parameter = header.this_parameter.or(synthesized_this).or(receiver_type);
        let template = self.signature_template(header.template, owner)?;

        // intern the signature of the walked header with its declared park color
        let parks = self.declaration_parks(source)?;
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            parks,
            asynchrony: signature.asynchrony,
            template,
            arguments: dir::TypeListId::EMPTY,
            this_parameter,
            parameters,
            return_type,
            is_generator: signature.is_generator,
            is_construct: false,
        };

        self.intern_signature(function)
    }

    /// Elide the result region from the receiver, one input, a union of inputs, or the default.
    pub(in crate::sema) fn apply_result_elision(
        &mut self,
        source: dir::LocalNodeIdAny,
        this_parameter: Option<dir::GlobalTypeId>,
        receiver_type: Option<dir::GlobalTypeId>,
        parameters: &[dir::FunctionParameterType],
        return_type: Option<dir::GlobalTypeId>,
        tracked: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Option<dir::GlobalTypeId>)> {
        // take the result as declared when it tracks no elision
        let Some(mut return_type) = return_type else {
            return Ok((None, None));
        };
        if tracked.is_empty() {
            return Ok((Some(return_type), None));
        }

        // collect the input regions, preferring the receiver's regions
        let mut seen = FxIndexSet::default();
        let mut input_regions = Vec::new();
        if let Some(this_parameter) = this_parameter.or(receiver_type) {
            input_regions.extend(self.input_regions(this_parameter)?);
            input_regions.retain(|region| seen.insert(*region));
        }
        if input_regions.is_empty() {
            for parameter in parameters {
                input_regions.extend(self.input_regions(parameter.ty)?);
            }
            input_regions.retain(|region| seen.insert(*region));
        }

        // synthesize a readonly receiver borrow for an elided result region
        let mut synthesized_this = None;
        if input_regions.is_empty()
            && this_parameter.is_none()
            && let Some(receiver) = receiver_type
        {
            let region = self.induce_signature_region(source)?;
            let access = self.access_literal(dir::Access::Readonly)?;
            let borrowed = self.check.borrow_value(region, access, receiver)?;
            synthesized_this = Some(borrowed);
            input_regions.extend(self.input_regions(borrowed)?);
            input_regions.retain(|region| seen.insert(*region));
        }

        // elect complete regions so each lifetime keeps its space
        let elected = match input_regions.as_slice() {
            [] => {
                let extent = self.lifetime_literal(dir::Lifetime::Managed)?;
                let place = self.local_space()?;
                self.intern_region(extent, place)?
            }
            [region] => *region,
            _ => self.normalized_union_type(input_regions.clone())?,
        };

        // rewrite each tracked return region to the elected one
        for region in tracked {
            return_type = self.check.replace_type(return_type, region, elected)?;
        }

        Ok((Some(return_type), synthesized_this))
    }

    /// Collect each complete borrow region in an input type.
    fn input_regions(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // collect each written region term whole
        let mut terms = Vec::new();
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let node = self.check.ty_raw(id)?;

            // follow solved variables toward their region terms
            if let dir::Type::Variable(variable) = node {
                if let Some(solution) = self.check.infer.solution(variable)? {
                    pending.push(solution);
                }

                continue;
            }

            // collect closed terms of the kind whole
            if self.check.memory_kind(id)? == Some(dir::MemoryParameter::Region) {
                terms.push(id);
            } else {
                self.check
                    .for_each_type_child(id.module_id, &node, |child| pending.push(child))?;
            }
        }

        Ok(terms)
    }

    /// Return the template owned by one callable signature.
    fn signature_template(
        &mut self,
        template: Option<GenericTemplateId>,
        owner: Option<InducedParameterOwner>,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if template.is_some() {
            return Ok(template);
        }
        let Some(owner) = owner else {
            return Ok(None);
        };

        Ok(self.check.template_by_source(owner.declaration))
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
        let signature = self.walk_callable_type(
            source,
            declaration.declares_generic_scope(),
            &declaration.generic_parameters,
            &declaration.where_clauses,
            declaration.this_parameter,
            &declaration.parameters,
            declaration.return_type,
            return_type,
            false,
        )?;
        let receiver = self.check.elided_receiver()?;

        self.push_function_value_type(signature, receiver)
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
        self.walk_callable_type(
            source,
            declaration.declares_generic_scope(),
            &declaration.generic_parameters,
            &declaration.where_clauses,
            None,
            &declaration.parameters,
            declaration.return_type,
            return_type,
            true,
        )
    }

    /// Walk one callable type annotation into its signature, eliding regions on its own binder.
    fn walk_callable_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        declares_generic_scope: bool,
        generic_parameters: &[dir::LocalNodeId<dir::GenericParameter>],
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
        return_annotation: Option<dir::LocalNodeId<dir::TypeExpression>>,
        return_type: Option<dir::GlobalTypeId>,
        is_construct: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let template = self.walk_signature_template(
            source,
            declares_generic_scope,
            generic_parameters,
            where_clauses,
        )?;

        // bind elided borrow regions on the callable type's own binder
        let owner = InducedParameterOwner::new(source.into_global(self.module), None, None);
        let previous = self.binder_owner.replace(owner);
        let signature = self.with_elision(ElisionSite::Signature, |walk| {
            // take the expected result, or walk the written one
            let (return_type, tracked) = match (return_type, return_annotation) {
                (Some(return_type), _) => (Some(return_type), Vec::new()),
                (None, Some(return_type)) => {
                    let (return_type, tracked) = walk.walk_return_type_expression(return_type)?;

                    (Some(return_type), tracked)
                }
                (None, None) => (None, Vec::new()),
            };

            // walk the explicit receiver parameter
            let this_parameter = match this_parameter {
                Some(parameter) => walk.walk_parameter_type(parameter)?,
                None => None,
            };

            // collect signature parameters
            let mut walked = Vec::new();
            for parameter in parameters {
                if let Some(parameter) = walk.walk_signature_parameter_type(*parameter)? {
                    walked.push(parameter);
                }
            }

            // elide the result region
            let (return_type, _) = walk.apply_result_elision(
                source,
                this_parameter,
                None,
                &walked,
                return_type,
                tracked,
            )?;

            Ok((this_parameter, walked, return_type))
        });
        self.binder_owner = previous;
        let (this_parameter, walked, return_type) = signature?;

        // close the induced template
        let template = self.induced_owner_template(owner, template)?;

        // intern the walked signature
        let parameters = self.intern_parameters(&walked)?;
        self.intern_signature(dir::FunctionSignatureType {
            parks: false,
            asynchrony: dir::Asynchrony::Sync,
            template,
            arguments: dir::TypeListId::EMPTY,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
            is_construct,
        })
    }

    /// Return one fat callable value type over a signature, taking its receiver in one term.
    pub(in crate::sema) fn push_function_value_type(
        &mut self,
        signature: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let function = dir::FunctionType {
            signature,
            receiver,
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

        // open the template the signature declares
        let source = source.into_global(self.module);
        let template = if generic_parameters.is_empty() {
            self.check
                .open_generic_template(source, self.flow().template_scope())?
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
        enclosing_receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<()> {
        // open the body scope and its result targets
        let source = body.into_any();
        let origin = Origin::Node(
            body.into_global_any(self.module),
            self.check.symbol_template(symbol)?,
        );
        let template = self.check.symbol_template(symbol)?;
        self.with_template_scope(template, |walk| {
            let mut return_target = result;
            let mut yield_target = None;
            let mut resume_target = None;

            // open the async completion type
            if signature.asynchrony == dir::Asynchrony::Async && !signature.is_generator {
                // infer an unannotated async function as a promise
                if signature.return_type.is_none() {
                    let completed = walk.open_type_hole(source, VariableKind::Type)?;
                    let promised = walk.language_type(dir::LanguageItem::Promise, &[completed])?;
                    let Some(variable) = walk.check.root_variable(result)? else {
                        return Err(CompilerError::Internal {
                            message: "inferred async return is not an inference variable".into(),
                        });
                    };
                    walk.check.commit_solution(variable, promised)?;
                    return_target = completed;
                }
                // retain the declared Promise, Task, or transparent owner
                else if let Some(completed) = walk.check.async_completion_type(origin, result)? {
                    return_target = completed;
                }
                // reject every other declared async result through the established relation
                else {
                    let completed =
                        walk.intern_operation(dir::TypeOperation::Awaited(dir::UnaryType {
                            target: result,
                        }))?;
                    let promised = walk.language_type(dir::LanguageItem::Promise, &[completed])?;
                    walk.relate_type(
                        origin,
                        CauseKind::Return { annotation: None },
                        Relation::Storable,
                        promised,
                        result,
                    )?;
                }
            }

            // open the generator yielded, completed, and resumed types
            if signature.is_generator {
                let yielded = walk.open_type_hole(source, VariableKind::Type)?;
                let completed = walk.open_type_hole(source, VariableKind::Type)?;
                let resumed = walk.open_type_hole(source, VariableKind::Type)?;
                let item = match signature.asynchrony {
                    // function* f() {}
                    dir::Asynchrony::Sync => dir::LanguageItem::Generator,
                    // async function* f() {}
                    dir::Asynchrony::Async => dir::LanguageItem::AsyncGenerator,
                };
                let generated = walk.language_type(item, &[yielded, completed, resumed])?;
                if signature.return_type.is_none() {
                    let Some(variable) = walk.check.root_variable(result)? else {
                        return Err(CompilerError::Internal {
                            message: "inferred generator return is not an inference variable"
                                .into(),
                        });
                    };
                    walk.check.commit_solution(variable, generated)?;
                } else {
                    walk.relate_type(
                        origin,
                        CauseKind::Return { annotation: None },
                        Relation::Storable,
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

            // enter the body node alone
            let body_site = match walk.tree.get(body) {
                dir::Expression::Block(block) => walk.enter_node(*block)?,
                _ => walk.enter_node(body)?,
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
                enclosing_receiver,
                entries,
                flow: None,
            };
            let form = match (body.asynchrony, &body.generator, body.return_type) {
                (dir::Asynchrony::Async, None, Some(completed)) => {
                    Some(CoroutineForm::Async { completed })
                }
                (_, Some(generator), Some(completed)) => Some(CoroutineForm::Generator {
                    yielded: generator.yielded,
                    completed,
                    resumed: generator.resumed,
                }),
                _ => None,
            };
            if let Some(form) = form {
                walk.check.coroutines.push(CoroutineBody {
                    symbol,
                    asynchrony: body.asynchrony,
                    form,
                });
            }
            if walk.check.functions.insert(symbol, body).is_some() {
                return Err(CompilerError::Internal {
                    message: format!("function {symbol:?} has multiple checked bodies"),
                });
            }

            Ok(())
        })
    }

    /// Return the signature slot for one walked runtime parameter.
    pub(in crate::sema) fn function_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::FunctionParameterType> {
        let parameter = self.tree.get(id);
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );

        // defaulted parameters may be omitted at the call site
        let is_optional = parameter.is_optional() || parameter.default_value().is_some();
        let name = parameter.name();

        Ok(dir::FunctionParameterType {
            name,
            ty,
            is_optional,
            is_rest,
        })
    }

    /// Walk one callable type parameter and return its signature slot.
    fn walk_signature_parameter_type(
        &mut self,
        id: dir::LocalNodeId<dir::Parameter>,
    ) -> CompilerResult<Option<dir::FunctionParameterType>> {
        // require a written annotation on every callable type parameter
        let parameter = self.tree.get(id);
        if parameter.declared_type().is_none() {
            self.check
                .report_missing_type_annotation(self.module, id.into_any());
        }

        // read the slot shape the parameter declares
        let is_rest = matches!(
            parameter,
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
        );
        let is_optional = parameter.is_optional();
        let name = parameter.name();
        let Some(ty) = self.walk_parameter_type(id)? else {
            return Ok(None);
        };

        // bind the parameter symbol through the one parameter binder
        self.bind_named_parameter(id, ty)?;

        Ok(Some(dir::FunctionParameterType {
            name,
            ty,
            is_optional,
            is_rest,
        }))
    }

    /// Walk one parameter annotation and return its argument type.
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
            let parameter = self.tree.get(id);
            let is_optional = parameter.is_optional() || parameter.default_value().is_some();
            let ty = if matches!(
                parameter,
                dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
            ) {
                self.walk_rest_type_expression(declared_type)?
            } else {
                self.walk_value_type(declared_type)?
            };

            // optional parameters accept explicit undefined at call sites
            if is_optional {
                self.optional_value_type(ty)?
            } else {
                ty
            }
        } else {
            self.open_type_hole(id.into_any(), VariableKind::Type)?
        };
        self.commit_node_type(id, ty)?;

        Ok(Some(ty))
    }
}
