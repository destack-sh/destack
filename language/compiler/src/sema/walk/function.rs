use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    CauseKind, CoroutineBody, CoroutineForm, FunctionBody, GeneratorTargets, GenericTemplateId,
    InducedParameterOwner, Origin, ReceiverBinding, Relation, VariableKind, WalkState,
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

        // intern the signature the walked header describes, with the park color its declaration carries
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
        tracked: Vec<dir::TypeVariableId>,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Option<dir::GlobalTypeId>)> {
        let Some(mut return_type) = return_type else {
            return Ok((None, None));
        };

        // collect the input regions, preferring the receiver's own
        let mut seen = FxIndexSet::default();
        let mut input_lifetimes = Vec::new();
        if let Some(this_parameter) = this_parameter.or(receiver_type) {
            input_lifetimes.extend(self.input_region_terms(this_parameter)?);
            input_lifetimes.retain(|lifetime| seen.insert(*lifetime));
        }
        if input_lifetimes.is_empty() {
            for parameter in parameters {
                input_lifetimes.extend(self.input_region_terms(parameter.ty)?);
            }
            input_lifetimes.retain(|lifetime| seen.insert(*lifetime));
        }

        // synthesize a readonly receiver borrow for an elided result region
        let mut synthesized_this = None;
        if input_lifetimes.is_empty()
            && this_parameter.is_none()
            && let Some(receiver) = receiver_type
            && !tracked.is_empty()
        {
            let region = self.induce_receiver_borrow_region(source)?;
            let access = self.access_literal(dir::Access::Readonly)?;
            let form = self.intern_borrow(region, access)?;
            let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
                form,
                value: receiver,
            }))?;
            synthesized_this = Some(borrowed);
            input_lifetimes.extend(self.input_region_terms(borrowed)?);
            input_lifetimes.retain(|lifetime| seen.insert(*lifetime));
        }

        // replace each tracked return region with the elected input
        if !tracked.is_empty() {
            // pick the result region: the unique input, the union of several, or managed storage
            let input_region = match input_lifetimes.as_slice() {
                [] => {
                    let extent = self.lifetime_literal(dir::Lifetime::Managed)?;
                    let spaces = self.check.place_literal(dir::Space::Local)?;

                    self.check.intern_region(extent, spaces)?
                }
                [region] => *region,
                _ => self.check.normalized_union_type(input_lifetimes.clone())?,
            };
            for return_variable in tracked {
                let return_region = self.check.variable_type(return_variable)?;
                return_type = self
                    .check
                    .replace_type(return_type, return_region, input_region)?;
            }
        }

        Ok((Some(return_type), synthesized_this))
    }

    /// Collect input region terms: open elided holes, written regions, and pairs.
    fn input_region_terms(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
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

            // collect closed region terms and pairs whole
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
        let template = self.walk_signature_template(
            source,
            declaration.declares_generic_scope(),
            &declaration.generic_parameters,
            &declaration.where_clauses,
        )?;

        // induce elided parameters on this function type's own template
        let owner = InducedParameterOwner::new(source.into_global(self.module), None, None);
        let previous = self.induced_owner.replace(owner);

        // take the expected result, or walk the written one
        let (return_type, tracked) = match (return_type, declaration.return_type) {
            (Some(return_type), _) => (Some(return_type), Vec::new()),
            (None, Some(return_type)) => {
                let (return_type, tracked) = self.walk_return_type_expression(return_type)?;

                (Some(return_type), tracked)
            }
            (None, None) => (None, Vec::new()),
        };

        // walk the written receiver parameter
        let this_parameter = match declaration.this_parameter {
            Some(parameter) => self.walk_parameter_type(parameter)?,
            None => None,
        };

        // collect signature parameters
        let mut parameters = Vec::new();
        for parameter in &declaration.parameters {
            if let Some(parameter) = self.walk_signature_parameter_type(*parameter)? {
                parameters.push(parameter);
            }
        }

        // elide the result region and close the induced template
        let (return_type, _) = self.apply_result_elision(
            source,
            this_parameter,
            None,
            &parameters,
            return_type,
            tracked,
        )?;
        self.induced_owner = previous;
        let template = self.induced_owner_template(owner, template)?;

        // intern the walked signature
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            parks: false,
            asynchrony: dir::Asynchrony::Sync,
            template,
            arguments: dir::TypeListId::EMPTY,
            this_parameter,
            parameters,
            return_type,
            is_generator: false,
            is_construct: false,
        };
        let signature = self.intern_signature(function)?;

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
        let template = self.walk_signature_template(
            source,
            declaration.declares_generic_scope(),
            &declaration.generic_parameters,
            &declaration.where_clauses,
        )?;
        // take the expected result, or walk the written one
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

        // elide the result region and intern the construct signature
        let (return_type, _) =
            self.apply_result_elision(source, None, None, &parameters, return_type, tracked)?;
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            parks: false,
            asynchrony: dir::Asynchrony::Sync,
            template,
            arguments: dir::TypeListId::EMPTY,
            this_parameter: None,
            parameters,
            return_type,
            is_generator: false,
            is_construct: true,
        };

        self.intern_signature(function)
    }

    /// Return one fat callable value type over a signature, taking its receiver in one term.
    pub(in crate::sema) fn push_function_value_type(
        &mut self,
        signature: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let place = self.check.local_place()?;
        let function = dir::FunctionType {
            signature,
            receiver,
            place,
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
                // infer unannotated async functions as promises
                if signature.return_type.is_none() {
                    let completed = walk.open_type_hole(source, VariableKind::Type)?;
                    let promised =
                        walk.language_type_reference(dir::LanguageItem::Promise, &[completed])?;
                    let Some(variable) = walk.check.root_variable(result)? else {
                        return Err(CompilerError::Internal {
                            message: "inferred async return is not an inference variable".into(),
                        });
                    };
                    walk.check.commit_solution(variable, promised)?;
                    return_target = completed;
                }
                // retain the declared Promise, Task, or transparent owner
                else if let Some(completed) = walk.check.async_completion_type(result)? {
                    return_target = completed;
                }
                // reject every other declared async result through the established relation
                else {
                    let completed =
                        walk.intern_operation(dir::TypeOperation::Awaited(dir::UnaryType {
                            target: result,
                        }))?;
                    let promised =
                        walk.language_type_reference(dir::LanguageItem::Promise, &[completed])?;
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
                let generated =
                    walk.language_type_reference(item, &[yielded, completed, resumed])?;
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
            let parameter = self.tree.get(id);
            let is_optional = parameter.is_optional() || parameter.default_value().is_some();
            let ty = if matches!(
                parameter,
                dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
            ) {
                self.walk_rest_type_expression(declared_type)?
            } else {
                self.walk_type_expression(declared_type)?
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
        if let Some(symbol) = self.declared_symbol(id.into_any()) {
            self.commit_symbol_type(symbol, binding)?;
        }

        Ok(Some(ty))
    }
}
