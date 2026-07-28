use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, CandidateVerdict, CheckFailure, CheckOutcome, Decision,
    DecisionKind, Dependency, Expectation, FlowSite, MemoryRank, NewtypeMatch, NewtypeOverload,
    NewtypeRejection, NewtypeSignature, Origin, SignatureMatch, SignatureRejection,
    SignatureSelection, TypeArgumentInference, TypeSubstitution, ValueCheck, ValueUse, answer,
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

        // return the committed construct target
        if let Some(target) = self.node_types.get(&source).copied() {
            return Ok(Answer::Ready(target));
        }

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
            self.commit_node_type(source, expected)?;

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
            self.commit_node_type(source, expected)?;

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

        // return the committed construct head
        if let Some(target) = self.node_types.get(&source).copied() {
            return Ok(Answer::Ready(target));
        }

        // non-reference heads keep strict annotation typing
        let dir::TypeExpression::Reference {
            path,
            generic_arguments,
            ..
        } = self.module(module).view().get(ty).clone()
        else {
            return Ok(Answer::Ready(self.require_node_type(source)?));
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
        let target = answer!(self.instantiate_construct_target(
            origin,
            source,
            symbol,
            &generic_arguments,
        )?);
        self.commit_node_type(source, target)?;

        Ok(Answer::Ready(target))
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
            let candidate = match self.reduce_type_head(origin, candidate)? {
                Answer::Ready(candidate) => candidate,
                Answer::Pending(blockers)
                    if blockers
                        .iter()
                        .all(|blocker| matches!(blocker, Dependency::Variable(_))) =>
                {
                    continue;
                }
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            match self.ty(candidate)? {
                dir::Type::Application(instance) if instance.symbol == symbol => {
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
            let ty = self.require_node_type(argument)?;
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
                dir::Type::Application(dir::GenericApplication { symbol, arguments }),
            )?;

            return Ok(Answer::Ready(target));
        };

        // infer omitted construct arguments
        let parameters = self.generic_template_parameters(template)?;
        let Some(substitution) = answer!(self.instantiate_parameters(
            origin,
            &parameters,
            &written,
            TypeSubstitution::default(),
            TypeArgumentInference::Exact,
        )?) else {
            let name = self.format_symbol(symbol);
            let expected = self.writable_parameter_count(&parameters);
            self.report_wrong_generic_arity(module, source.local_id, name, expected, written.len());
            let error = self.intern_type(module, dir::Type::Error)?;

            return Ok(Answer::Ready(error));
        };

        // register every bound and predicate on the constructed application
        for constraint in
            self.substitute_application_constraints(origin, template, &substitution)?
        {
            self.check.push_constraint(constraint);
        }

        let arguments = substitution.arguments().collect::<SmallVec<[_; 4]>>();
        let arguments = self.intern_type_ids(module, &arguments)?;
        let target = self.intern_type(
            module,
            dir::Type::Application(dir::GenericApplication { symbol, arguments }),
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
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // separate destination forms from the constructed instance
        let mut forms = SmallVec::<[dir::Form; 2]>::new();
        let mut expected_value = expectation.map(|expectation| expectation.target);
        while let Some(expected) = expected_value {
            let head = answer!(self.reduce_type_head(origin, expected)?);
            let dir::Type::Form(form) = self.ty(head)? else {
                break;
            };
            if !matches!(form.form, dir::Form::Owned | dir::Form::Placed { .. }) {
                break;
            }
            forms.push(form.form);
            expected_value = Some(form.value);
        }

        // select the constructed target
        let target = answer!(self.select_construct_target(site, ty, expected_value)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        let instance = match self.ty(target)? {
            dir::Type::Application(instance) => instance,
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };

        // require a class to construct through new
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
                    let error = self.commit_error_node(node)?;

                    return Ok(Answer::Ready(error));
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
        let expectation = receiver.or(expected_value).and_then(|target| {
            expectation.map(|expectation| Expectation {
                target,
                ..expectation
            })
        });

        // winnow constructors in declaration order, then confirm the winner
        let is_single_candidate = constructors.len() == 1;
        let mut winner: Option<(MemoryRank, dir::ClassConstructorDefinition)> = None;
        let mut indeterminate = None;
        let mut rejections = Vec::new();
        for constructor in constructors {
            if is_single_candidate {
                winner = Some((MemoryRank::Exact, constructor));
                break;
            }
            let mut rank = MemoryRank::Exact;
            let (verdict, rejection) = answer!(self.probe_candidate_noted(
                |state| {
                    let outcome = state.attempt_construct(
                        origin,
                        module,
                        target.module_id,
                        &instance,
                        target,
                        constructor.ty,
                        &arguments,
                        expectation,
                        receiver,
                    )?;
                    match outcome {
                        Answer::Ready(matched) => {
                            if let SignatureMatch::Selected(selection) = &matched {
                                rank = selection.rank;
                            }

                            Ok(Answer::Ready(matched.into_candidate()))
                        }
                        Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                    }
                },
                |state, rejection| {
                    state
                        .check
                        .describe_signature_rejection(module, constructor.ty, rejection)
                },
            )?);
            match verdict {
                CandidateVerdict::Rejected => rejections.extend(rejection),
                CandidateVerdict::Viable => {
                    if rank == MemoryRank::Exact {
                        winner = Some((rank, constructor));
                        break;
                    }
                    if winner.as_ref().is_none_or(|(best, _)| rank < *best) {
                        winner = Some((rank, constructor));
                    }
                }
                CandidateVerdict::Indeterminate => {
                    indeterminate.get_or_insert(constructor);
                }
            }
        }
        let winner = winner.map(|(_, constructor)| constructor);

        // confirm the winner outside any probe
        if let Some(constructor) = winner.or(indeterminate) {
            let attempt = self.attempt_construct(
                origin,
                module,
                target.module_id,
                &instance,
                target,
                constructor.ty,
                &arguments,
                expectation,
                receiver,
            )?;

            match answer!(attempt) {
                SignatureMatch::Selected(signature) | SignatureMatch::ReturnMismatch(signature) => {
                    return self.commit_construct(
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
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                } if is_single_candidate => {
                    self.report_signature_rejection(origin, rejection)?;
                    let produced = answer!(self.commit_construct(
                        node,
                        module,
                        target.module_id,
                        argument_nodes,
                        &instance,
                        constructor.constructor,
                        selection,
                        result,
                        &forms,
                    )?);

                    return Ok(Answer::Ready(produced));
                }
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    self.report_signature_rejection(origin, rejection)?;
                    self.commit_decision(node, Decision::Rejected)?;
                    let error = self.commit_error_node(node)?;

                    return Ok(Answer::Ready(error));
                }
                SignatureMatch::Invalid { .. } => {}
                SignatureMatch::Inapplicable(_) => {}
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
        instance: &dir::GenericApplication,
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
        let (extends_module, instance) = self.require_nominal_application(extends.ty)?;
        if active.contains(&instance.symbol) {
            return Ok(Answer::Ready(Vec::new()));
        }
        active.push(instance.symbol);

        let base = match self.definition(instance.symbol)? {
            Some(dir::Definition::Class(base)) => base.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("base class {:?} has no checked definition", instance.symbol),
                });
            }
        };
        let module = origin.module();
        let arguments = self.type_ids(extends_module, instance.arguments)?.to_vec();
        let arguments = self.intern_type_ids(module, &arguments)?;
        let instance = dir::GenericApplication {
            arguments,
            ..instance
        };
        let base_receiver = self.intern_type(module, dir::Type::Application(instance))?;
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
            let constructor = base_constructor.constructor.forwarded(instance.symbol);
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
        origin: Origin,
        module: ModuleId,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
        function_type: dir::GlobalTypeId,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
        // reduce the constructor shape before matching arguments
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            )));
        };
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol)?;
        if instance.arguments.is_empty() && template.is_some() {
            return self.match_signature(
                origin,
                module,
                function_type.module_id,
                Some(instance.symbol),
                &[],
                &[],
                &function,
                return_type,
                receiver,
                arguments,
                expectation,
            );
        }

        // applied classes substitute their written arguments
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(target);
        let function_type = self.substitute_type(origin.module(), function_type, &substitution)?;
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(Answer::Ready(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            )));
        };

        let return_type = function.return_type.or(Some(target));
        let carried = self.settled_argument_bindings(&substitution.bindings)?;
        self.match_signature(
            origin,
            module,
            function_type.module_id,
            Some(instance.symbol),
            &carried,
            &[],
            &function,
            return_type,
            receiver,
            arguments,
            expectation,
        )
    }

    /// Select one newtype construction.
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
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let matched = answer!(self.match_newtype(
            origin,
            symbol,
            argument_nodes,
            type_arguments,
            expectation,
            NewtypeOverload::Ordered,
            ValueUse::Argument,
        )?);
        let (signature, rejection, outcome) = match matched {
            NewtypeMatch::Selected(signature) => (signature, None, CheckOutcome::Holds),
            NewtypeMatch::ReturnMismatch(signature) => {
                (signature, None, CheckOutcome::Fails(CheckFailure::Relation))
            }
            NewtypeMatch::Invalid {
                signature,
                rejection,
            } => {
                self.report_signature_rejection(origin, rejection)?;

                (signature, None, CheckOutcome::Fails(CheckFailure::Relation))
            }
            NewtypeMatch::Rejected(rejection) => match rejection {
                NewtypeRejection::Signature(rejection) => {
                    self.report_signature_rejection(origin, rejection)?;
                    self.commit_decision(node, Decision::Rejected)?;
                    let source = self.commit_error_node(node)?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(Answer::Ready(ValueCheck {
                        source,
                        outcome: CheckOutcome::Fails(CheckFailure::Relation),
                        target,
                    }));
                }
                NewtypeRejection::NoMatch(notes) => {
                    let source = answer!(self.reject_construct(
                        site,
                        node,
                        origin,
                        argument_nodes,
                        &notes,
                    )?);
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(Answer::Ready(ValueCheck {
                        source,
                        outcome: CheckOutcome::Fails(CheckFailure::Relation),
                        target,
                    }));
                }
                NewtypeRejection::Ambiguous => {
                    return Err(CompilerError::Internal {
                        message: "ordered newtype selection rejected an ambiguous backing"
                            .to_string(),
                    });
                }
            },
        };

        // report the rejected invocation
        let module = origin.module();
        if let Some(rejection) = rejection {
            self.report_signature_rejection(origin, rejection)?;
        }

        // commit the selected newtype construction
        let NewtypeSignature {
            selection,
            parameters,
            coercions,
            return_type,
        } = signature;
        // commit conversions only after the backing has been selected
        for (source, coercion) in &coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }
        let target = dir::ConstructTarget::Newtype(selection);
        let resolution = dir::ConstructResolution::new(
            target,
            self.argument_bindings(module, argument_nodes, &parameters),
            return_type,
        );
        self.commit_decision(node, Decision::Construct(resolution))?;
        self.commit_node_type(node, return_type)?;

        let expected = expectation.map_or(return_type, |expectation| expectation.target);

        Ok(Answer::Ready(ValueCheck {
            source: return_type,
            outcome,
            target: expected,
        }))
    }

    /// Commit one accepted construction selection.
    fn commit_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        instance_module: ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        instance: &dir::GenericApplication,
        constructor: dir::ClassConstructor,
        signature: SignatureSelection,
        result: ConstructResult,
        forms: &[dir::Form],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let generic_arguments = if signature.generic_arguments.is_empty() {
            let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };
        // commit conversions only after the constructor has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }
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
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            produced,
        );
        self.commit_decision(node, Decision::Construct(resolution))?;

        self.commit_node_type(node, produced)?;

        Ok(Answer::Ready(produced))
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
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let arguments = answer!(self.infer_argument_types(site, argument_nodes)?);
        self.report_no_matching_construct(origin, &arguments, rejections)?;
        self.commit_decision(node, Decision::Rejected)?;
        let error = self.commit_error_node(node)?;

        Ok(Answer::Ready(error))
    }

    /// Reject one construction whose target cannot use new.
    fn reject_not_constructible(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
        hint: &str,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        self.report_not_constructible(origin, target, hint)?;
        self.commit_decision(node, Decision::Rejected)?;
        let error = self.commit_error_node(node)?;

        Ok(Answer::Ready(error))
    }
}
