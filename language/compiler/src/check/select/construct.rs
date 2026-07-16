use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, CandidateOutcome, CandidatePass, CandidateVerdict, Cause,
    CauseKind, Decision, DecisionKind, FlowSite, Origin, ProbeReason, Relation, SignatureRejection,
    SignatureSelection, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

/// Result shape produced by one construct expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum ConstructResult {
    /// Construct expression produces the constructed value directly.
    Direct,
    /// Construct expression produces the fallible construction carrier.
    Fallible,
}

impl BodyState<'_, '_> {
    /// Select the target type named by one construct head.
    pub(in crate::check) fn select_construct_target(
        &mut self,
        site: FlowSite,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = site.node.module_id;
        let origin = site.origin();
        let source = ty.into_global_any(module);

        // omitted heads are owned entirely by the expected target
        if matches!(
            self.module(module).view().get(ty),
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                ..
            }
        ) {
            let Some(expected) = answer!(self.expected_construct_target(origin, expected)?) else {
                self.report_cannot_infer_node(site.node)?;
                let error = self.intern_type(module, dir::Type::Error)?;

                return Ok(Answer::Ready(error));
            };

            return Ok(Answer::Ready(expected));
        }

        // a uniquely matching contextual arm supplies omitted generic arguments
        if let dir::TypeExpression::Reference {
            generic_arguments, ..
        } = self.module(module).view().get(ty)
            && generic_arguments.is_empty()
            && let Some(expected) = expected
            && let Some(resolution) = self.resolutions(source.module_id).name_resolution(source)
            && let [symbol] = resolution.symbols()
            && let Some(expected) =
                answer!(self.expected_construct_instance(origin, expected, *symbol)?)
        {
            return Ok(Answer::Ready(expected));
        }

        self.written_construct_tag(origin, module, ty)
    }

    /// Return the written nominal tag type for one construct or pattern head.
    pub(in crate::check) fn written_construct_tag(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let source = ty.into_global_any(module);

        // non-reference heads keep strict annotation typing
        let dir::TypeExpression::Reference {
            path,
            generic_arguments,
            ..
        } = self.module(module).view().get(ty).clone()
        else {
            return self.node_type(source);
        };

        // read the construct declaration captured during walk
        let name = self
            .resolutions(source.module_id)
            .name_resolution(source)
            .cloned();
        let symbol = match (self.decision_kind(source), name) {
            (Some(DecisionKind::Name), Some(resolution)) => match resolution.symbols() {
                [symbol] => *symbol,
                _ => {
                    self.report_ambiguous_reference(module, ty.into_any(), &path);
                    let error = self.intern_type(module, dir::Type::Error)?;

                    return Ok(Answer::Ready(error));
                }
            },
            (Some(DecisionKind::Rejected), _) => {
                let error = self.intern_type(module, dir::Type::Error)?;

                return Ok(Answer::Ready(error));
            }
            (Some(other), _) => {
                return Err(CompilerError::Internal {
                    message: format!("construct target {source:?} decided as {other:?}"),
                });
            }
            // reference heads decide during the walk
            (None, _) => {
                return Err(CompilerError::Internal {
                    message: format!("construct target {source:?} has no walk decision"),
                });
            }
        };

        // instantiate written arguments and open omitted construct parameters
        self.instantiate_construct_target(origin, source, symbol, &generic_arguments)
    }

    /// Return the expected target after peeling construction forms.
    fn expected_construct_target(
        &mut self,
        origin: Origin,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(expected) = expected else {
            return Ok(Answer::Ready(None));
        };
        let target = answer!(self.reduce_type_head(origin, expected)?);
        let target = match self.ty(target)? {
            dir::Type::Form(form) if form.form == dir::Form::Owned => form.value,
            _ => target,
        };

        Ok(Answer::Ready(Some(target)))
    }

    /// Return the unique contextual instance of one construct declaration.
    fn expected_construct_instance(
        &mut self,
        origin: Origin,
        expected: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(&[expected]);
        let mut matched = None;

        // search direct, owned, and union targets for one matching nominal head
        while let Some(candidate) = pending.pop() {
            let candidate = answer!(self.reduce_type_head(origin, candidate)?);
            match self.ty(candidate)? {
                dir::Type::Instance(instance) if instance.symbol == symbol => {
                    if matched.is_some() {
                        return Ok(Answer::Ready(None));
                    }
                    matched = Some(candidate);
                }
                dir::Type::Form(form) if form.form == dir::Form::Owned => {
                    pending.push(form.value);
                }
                dir::Type::Union(union) => {
                    pending.extend_from_slice(self.type_ids(candidate.module_id, union.elements)?)
                }
                _ => {}
            }
        }

        Ok(Answer::Ready(matched))
    }

    /// Instantiate one construct head with its written generic arguments.
    fn instantiate_construct_target(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = source.module_id;
        let mut written = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in arguments {
            let argument = argument.into_global_any(module);
            let ty = answer!(self.node_type(argument)?);
            written.push(ty);
        }

        // reject written arguments on nongeneric heads
        let Some(template) = self.symbol_template(symbol)? else {
            if !written.is_empty() {
                let name = self.format_symbol(symbol);
                self.report_wrong_generic_arity(module, source.local_id, name, 0, written.len());
                let error = self.intern_type(module, dir::Type::Error)?;

                return Ok(Answer::Ready(error));
            }
            let arguments = self.intern_type_ids(module, &[])?;
            let target = self.intern_type(
                module,
                dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
            )?;

            return Ok(Answer::Ready(target));
        };

        // infer omitted construct arguments
        let parameters = self.generic_template_parameters(template);
        let Some(substitution) = self.instantiate_parameter_arguments(
            origin,
            &parameters,
            &written,
            TypeSubstitution::default(),
        )?
        else {
            let name = self.format_symbol(symbol);
            let expected = self.written_parameter_count(&parameters);
            self.report_wrong_generic_arity(module, source.local_id, name, expected, written.len());
            let error = self.intern_type(module, dir::Type::Error)?;

            return Ok(Answer::Ready(error));
        };

        // check selected arguments against declared bounds
        let sources = SmallVec::<[dir::GlobalNodeIdAny; 4]>::from_iter(std::iter::repeat_n(
            source,
            substitution.arguments.len(),
        ));
        if let Some(rejection) = answer!(self.check_generic_arguments(
            origin,
            &parameters,
            &substitution.arguments,
            &sources,
            &substitution,
        )?) {
            let anchored = self.origin_at(origin, rejection.source)?;
            let anchored = self
                .check
                .intern_cause(Cause::root(anchored, CauseKind::Expression));
            self.relate(
                anchored,
                Relation::Satisfies,
                None,
                rejection.argument,
                rejection.bound,
            )?;
            let error = self.intern_type(module, dir::Type::Error)?;

            return Ok(Answer::Ready(error));
        }

        let arguments = self.intern_type_ids(module, &substitution.arguments)?;
        let target = self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
        )?;

        Ok(Answer::Ready(target))
    }

    /// Select the construction meaning of one new expression.
    pub(in crate::check) fn select_construct(
        &mut self,
        site: FlowSite,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        result: ConstructResult,
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        let arguments = self.callable_arguments(module, argument_nodes)?;

        // fresh constructions materialize at owned and placed expected forms,
        //  taking their storage form directly like widened literals
        let mut forms = SmallVec::<[dir::Form; 2]>::new();
        let mut expected_return = expected_return;
        while let Some(expected) = expected_return {
            let head = answer!(self.reduce_type_head(origin, expected)?);
            let dir::Type::Form(form) = self.ty(head)? else {
                break;
            };
            if !matches!(form.form, dir::Form::Owned | dir::Form::Placed { .. }) {
                break;
            }
            forms.push(form.form);
            expected_return = Some(form.value);
        }

        // select the constructed target
        let target = answer!(self.select_construct_target(site, ty, expected_return)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        let instance = match self.ty(target)? {
            dir::Type::Instance(instance) => instance,
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };

        // only classes construct through new
        let constructors = match self.definition(instance.symbol)? {
            Some(dir::Definition::Struct(_)) => {
                return self.reject_not_constructible(
                    node,
                    origin,
                    target,
                    "; construct value types with 'T { … }'",
                );
            }
            Some(dir::Definition::Newtype(_)) => {
                return self.reject_not_constructible(
                    node,
                    origin,
                    target,
                    "; construct newtypes with 'T(…)'",
                );
            }
            Some(dir::Definition::Class(definition)) => {
                if definition.is_abstract {
                    self.report_cannot_construct_abstract_type(origin, target)?;
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                }

                let constructors = definition.constructors.clone();
                let extends = definition.extends.clone();
                let mut active = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
                answer!(self.collect_class_construct_candidates(
                    origin,
                    target,
                    &instance,
                    constructors,
                    extends,
                    &mut active,
                )?)
            }
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };
        if constructors.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        }

        // the destination place resolves relative constructor member types
        let receiver = forms
            .iter()
            .find_map(|form| match form {
                dir::Form::Placed { place } => Some(*place),
                _ => None,
            })
            .map(|place| {
                self.intern_type(
                    module,
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Placed { place },
                        value: target,
                    }),
                )
            })
            .transpose()?;
        // signatures expect the constructed instance in its destination place
        let expected_return = receiver.or(expected_return);

        // winnow constructors in declaration order, then confirm the winner
        let is_single_candidate = constructors.len() == 1;
        let mut winner = None;
        let mut ambiguous = None;
        let mut rejections = Vec::new();
        for constructor in constructors {
            if is_single_candidate {
                winner = Some(constructor);
                break;
            }
            let (verdict, rejection) = self.probe_candidate_noted(
                ProbeReason::Signature,
                |state| {
                    state.attempt_construct(
                        CandidatePass::Winnow,
                        origin,
                        module,
                        target.module_id,
                        &instance,
                        target,
                        constructor.ty,
                        &arguments,
                        expected_return,
                        receiver,
                    )
                },
                |state, rejection| {
                    Ok(state
                        .check
                        .describe_signature_rejection(module, constructor.ty, rejection))
                },
            )?;
            match verdict {
                CandidateVerdict::Rejected => rejections.extend(rejection),
                CandidateVerdict::Viable => {
                    winner = Some(constructor);
                    break;
                }
                CandidateVerdict::Ambiguous => {
                    ambiguous.get_or_insert(constructor);
                }
            }
        }

        // confirm the winner outside any probe
        if let Some(constructor) = winner.or(ambiguous) {
            let attempt = self.attempt_construct(
                CandidatePass::Confirm,
                origin,
                module,
                target.module_id,
                &instance,
                target,
                constructor.ty,
                &arguments,
                expected_return,
                receiver,
            )?;

            if let CandidateOutcome::Accepted(signature) = answer!(attempt) {
                return self.commit_construct(
                    site,
                    node,
                    module,
                    target.module_id,
                    argument_nodes,
                    &instance,
                    constructor.constructor,
                    signature,
                    result,
                    &forms,
                );
            }
        }

        rejections.truncate(4);
        self.reject_construct(site, node, origin, argument_nodes, &rejections)
    }

    /// Return construct candidates for one class instance.
    fn collect_class_construct_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        constructors: Vec<dir::ClassConstructorDefinition>,
        extends: Option<dir::NominalHeritage>,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Answer<Vec<dir::ClassConstructorDefinition>>> {
        if !constructors.is_empty() {
            return Ok(Answer::Ready(constructors));
        }

        let Some(extends) = extends else {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        };

        self.collect_forwarded_class_construct_candidates(origin, receiver, &extends, active)
    }

    /// Return constructors forwarded from one base class.
    fn collect_forwarded_class_construct_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        extends: &dir::NominalHeritage,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Answer<Vec<dir::ClassConstructorDefinition>>> {
        if active.contains(&extends.symbol) {
            return Ok(Answer::Ready(Vec::new()));
        }
        active.push(extends.symbol);

        let base = match self.definition(extends.symbol)? {
            Some(dir::Definition::Class(base)) => base.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("base class {:?} has no checked definition", extends.symbol),
                });
            }
        };
        let module = origin.module();
        let arguments = self.intern_type_ids(module, &extends.arguments)?;
        let instance = dir::GenericInstance {
            symbol: extends.symbol,
            arguments,
        };
        let base_receiver = self.intern_type(module, dir::Type::Instance(instance))?;
        let base_constructors = answer!(self.collect_class_construct_candidates(
            origin,
            base_receiver,
            &instance,
            base.constructors,
            base.extends,
            active,
        )?);
        active.pop();

        let substitution = self
            .instance_substitution(module, &instance)?
            .with_receiver(receiver);
        let mut constructors = Vec::with_capacity(base_constructors.len());
        for base_constructor in base_constructors {
            let constructor = base_constructor.constructor.forwarded(extends.symbol);
            let ty = self.substitute_type(origin.module(), base_constructor.ty, &substitution)?;
            let ty = self.class_constructor_returning(origin, ty, receiver)?;

            constructors.push(dir::ClassConstructorDefinition { constructor, ty });
        }

        Ok(Answer::Ready(constructors))
    }

    /// Return one constructor signature with a replaced return type.
    fn class_constructor_returning(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(mut function) = self.signature_head(ty)? else {
            return Err(CompilerError::Internal {
                message: format!("class constructor type {ty:?} is not a function signature"),
            });
        };
        function.return_type = Some(receiver);

        self.intern_signature(origin.module(), function)
    }

    /// Attempt one constructor candidate against collected arguments.
    fn attempt_construct(
        &mut self,
        pass: CandidatePass,
        origin: Origin,
        module: ModuleId,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
        function_type: dir::GlobalTypeId,
        arguments: &[CallableArgument],
        expected_return: Option<dir::GlobalTypeId>,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<CandidateOutcome<SignatureSelection, SignatureRejection>>> {
        let source = self.origin_source_node(origin)?;

        // reduce the constructor shape before matching arguments
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(Answer::Ready(CandidateOutcome::Rejected(
                SignatureRejection::Inapplicable,
            )));
        };
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol)?;
        if instance.arguments.is_empty()
            && let Some(template) = template
        {
            let parameters = self.generic_template_parameters(template);
            return self.match_signature(
                pass,
                origin,
                module,
                function_type.module_id,
                source,
                &parameters,
                None,
                &[],
                &[],
                &function,
                return_type,
                receiver,
                arguments,
                expected_return,
            );
        }

        // applied classes substitute their written arguments
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(target);
        let function_type = self.substitute_type(origin.module(), function_type, &substitution)?;
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(Answer::Ready(CandidateOutcome::Rejected(
                SignatureRejection::Inapplicable,
            )));
        };

        let return_type = function.return_type.or(Some(target));
        let carried =
            self.generic_argument_bindings(&substitution.parameters, &substitution.arguments)?;
        self.match_signature(
            pass,
            origin,
            module,
            function_type.module_id,
            source,
            &[],
            None,
            &carried,
            &[],
            &function,
            return_type,
            receiver,
            arguments,
            expected_return,
        )
    }

    /// Select one newtype construction through call expression form.
    ///
    /// Example:
    /// ```ds
    /// UserId(1)
    /// Point(1, 2)
    /// ```
    pub(in crate::check) fn select_newtype_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let arguments = self.callable_arguments(module, argument_nodes)?;

        // read the wrapped backing type
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol)? else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };
        let backing = definition.value;

        // model the backing as a callable signature
        let generic_parameters = self
            .symbol_template(symbol)?
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        let type_arguments = if type_arguments.is_empty() {
            answer!(self.expected_newtype_arguments(origin, symbol, expected_return)?)
        } else {
            type_arguments.to_vec()
        };
        let return_arguments = generic_parameters
            .iter()
            .copied()
            .map(|parameter| self.intern_type(module, dir::Type::Parameter(parameter)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let return_arguments = self.intern_type_ids(module, &return_arguments)?;
        let return_type = self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: return_arguments,
            }),
        )?;
        let backing = answer!(self.reduce_type_head(origin, backing)?);
        let parameters = match self.ty(backing)? {
            dir::Type::Tuple(tuple) => self
                .tuple_elements(backing.module_id, tuple.elements)?
                .iter()
                .map(|element| dir::FunctionParameterType {
                    ty: element.ty,
                    is_optional: false,
                    is_rest: false,
                })
                .collect::<Vec<_>>(),
            _ => vec![dir::FunctionParameterType {
                ty: backing,
                is_optional: false,
                is_rest: false,
            }],
        };
        let parameters = self.intern_parameters(module, &parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
        };
        let attempt = self.match_signature(
            CandidatePass::Confirm,
            origin,
            module,
            module,
            source,
            &generic_parameters,
            None,
            &[],
            &type_arguments,
            &function,
            function.return_type,
            None,
            &arguments,
            expected_return,
        )?;
        let signature = match answer!(attempt) {
            CandidateOutcome::Accepted(signature) => signature,
            CandidateOutcome::Rejected(_) => {
                return self.reject_construct(site, node, origin, argument_nodes, &[]);
            }
        };

        // commit the selected newtype construction
        let target = dir::ConstructTarget::Newtype(dir::NewtypeConstructCandidate {
            symbol,
            generic_arguments: signature.generic_arguments.clone(),
        });
        let resolution = dir::ConstructResolution::new(
            target,
            Self::parameter_types(&signature.parameters),
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            signature.return_type,
        );
        self.check_arguments(site, argument_nodes, &resolution.arguments)?;
        self.commit_decision(node, Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;

        Ok(Answer::Ready(()))
    }

    /// Return implicit newtype arguments from a same-symbol expected result.
    fn expected_newtype_arguments(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Vec<dir::GlobalTypeId>>> {
        let Some(expected_return) = expected_return else {
            return Ok(Answer::Ready(Vec::new()));
        };
        let expected_return = answer!(self.reduce_type_head(origin, expected_return)?);
        let dir::Type::Instance(instance) = self.ty(expected_return)? else {
            return Ok(Answer::Ready(Vec::new()));
        };
        if instance.symbol != symbol {
            return Ok(Answer::Ready(Vec::new()));
        }

        Ok(Answer::Ready(
            self.type_ids(expected_return.module_id, instance.arguments)?
                .to_vec(),
        ))
    }

    /// Commit one accepted construction selection.
    fn commit_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        instance_module: ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        instance: &dir::GenericInstance,
        constructor: dir::ClassConstructor,
        signature: SignatureSelection,
        result: ConstructResult,
        forms: &[dir::Form],
    ) -> CompilerResult<Answer<()>> {
        let generic_arguments = if signature.generic_arguments.is_empty() {
            let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };
        let target = dir::ConstructTarget::Class(dir::ClassConstructCandidate {
            symbol: instance.symbol,
            constructor,
            generic_arguments,
        });
        let mut produced = match result {
            ConstructResult::Direct => signature.return_type,
            ConstructResult::Fallible => {
                self.fallible_construct_type(node, signature.return_type)?
            }
        };
        for form in forms.iter().rev().copied() {
            produced = self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form,
                    value: produced,
                }),
            )?;
        }
        let resolution = dir::ConstructResolution::new(
            target,
            Self::parameter_types(&signature.parameters),
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            produced,
        );
        self.check_arguments(site, argument_nodes, &resolution.arguments)?;
        self.commit_decision(node, Decision::Construct(resolution))?;

        self.commit_node_type(node, produced)?;

        Ok(Answer::Ready(()))
    }

    /// Return the result carrier for one fallible construction.
    fn fallible_construct_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let allocation_error =
            self.language_type(node.module_id, dir::LanguageItem::AllocationError, &[])?;
        let carrier = self.language_type(
            node.module_id,
            dir::LanguageItem::Result,
            &[value, allocation_error],
        )?;

        Ok(carrier)
    }

    /// Reject one construction whose arguments fit no constructor.
    pub(in crate::check) fn reject_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        rejections: &[String],
    ) -> CompilerResult<Answer<()>> {
        let arguments = answer!(self.infer_argument_types(site, argument_nodes)?);
        self.report_no_matching_construct(origin, &arguments, rejections)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }

    /// Reject one construction whose target cannot use new.
    fn reject_not_constructible(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
        hint: &str,
    ) -> CompilerResult<Answer<()>> {
        self.report_not_constructible(origin, target, hint)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}
