use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    BodyState, CallableArgument, CandidateVerdict, CheckFailure, CheckOutcome, Expectation,
    FlowSite, NewtypeMatch, NewtypeOverload, NewtypeRejection, NewtypeSignature, Origin, PlaceUse,
    SignatureFamily, SignatureMatch, SignatureRejection, SignatureSelection, TypeArgumentInference,
    TypeSubstitution, ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select the target type named by one construct head.
    pub(in crate::check) fn select_construct_target(
        &mut self,
        site: FlowSite,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = site.node.module_id;
        let origin = site.origin();
        let source = ty.into_global_any(module);

        // walk the construct type at its first typing visit
        self.walk_body_construct_type(module, ty)?;

        // return the committed construct target
        if let Some(target) = self.node_types.get(&source) {
            return Ok(target);
        }

        // omitted heads are owned entirely by the expected target
        if matches!(
            self.module(module).view().get(ty),
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                ..
            }
        ) {
            let Some(expected) = self.expected_construct_target(origin, expected)? else {
                self.report_cannot_infer_node(site.node)?;
                let error = self.intern_type(dir::Type::Error)?;

                return Ok(error);
            };
            self.commit_node_type(source, expected)?;

            return Ok(expected);
        }

        // a uniquely matching contextual arm supplies omitted generic arguments
        if let dir::TypeExpression::Reference {
            generic_arguments, ..
        } = self.module(module).view().get(ty)
            && generic_arguments.is_empty()
            && let Some(expected) = expected
            && let Some(resolution) = self.resolutions(source.module_id).name_resolution(source)
            && let [symbol] = resolution.symbols()
            && let Some(expected) = self.expected_construct_instance(origin, expected, *symbol)?
        {
            self.commit_node_type(source, expected)?;

            return Ok(expected);
        }

        self.written_construct_tag(origin, module, ty)
    }

    /// Return the written nominal tag type for one construct or pattern head.
    pub(in crate::check) fn written_construct_tag(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = ty.into_global_any(module);

        // return the committed construct head
        if let Some(target) = self.node_types.get(&source) {
            return Ok(target);
        }

        // non-reference heads keep strict annotation typing
        let dir::TypeExpression::Reference {
            path,
            generic_arguments,
            ..
        } = self.module(module).view().get(ty).clone()
        else {
            return self.require_node_type(source);
        };

        // read the construct declaration captured during walk
        let symbol = match self.name_decision(source) {
            Some(resolution) => match resolution.symbols() {
                [symbol] => *symbol,
                _ => {
                    self.report_ambiguous_reference(module, ty.into_any(), &path);
                    let error = self.intern_type(dir::Type::Error)?;

                    return Ok(error);
                }
            },
            None => match self.decision(source).cloned() {
                Some(dir::Decision::Rejected | dir::Decision::Poisoned) => {
                    let error = self.intern_type(dir::Type::Error)?;

                    return Ok(error);
                }
                Some(other) => {
                    return Err(CompilerError::Internal {
                        message: format!("construct target {source:?} decided as {other:?}"),
                    });
                }
                // reference heads decide during the walk
                None => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "construct target {} has no walk decision",
                            self.node_label(source)
                        ),
                    });
                }
            },
        };

        // construct value bindings through their inferred value type
        if self
            .symbol_kind_maybe(symbol)?
            .is_some_and(dir::SymbolKind::is_binding)
        {
            let target = self.symbol_type(symbol)?;
            let target = self.strip_form(origin, target)?;
            self.commit_node_type(source, target)?;

            return Ok(target);
        }

        // instantiate written arguments and open omitted construct parameters
        let target =
            self.instantiate_construct_target(origin, source, symbol, &generic_arguments)?;
        self.commit_node_type(source, target)?;

        Ok(target)
    }

    /// Return the expected target after peeling construction forms.
    fn expected_construct_target(
        &mut self,
        origin: Origin,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(expected) = expected else {
            return Ok(None);
        };
        let target = self.reduce_type_head(origin, expected)?;
        let target = match self.ty(target)? {
            dir::Type::Form(form) if form.form == dir::Form::Owned => form.value,
            _ => target,
        };

        Ok(Some(target))
    }

    /// Return the unique contextual instance of one construct declaration.
    fn expected_construct_instance(
        &mut self,
        origin: Origin,
        expected: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(&[expected]);
        let mut matched = None;

        // search direct, owned, and union targets for one matching nominal head
        while let Some(candidate) = pending.pop() {
            let candidate = self.reduce_type_head(origin, candidate)?;
            match self.ty(candidate)? {
                dir::Type::Application(instance) if instance.symbol == symbol => {
                    if matched.is_some() {
                        return Ok(None);
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

        Ok(matched)
    }

    /// Instantiate one construct head with its written generic arguments.
    fn instantiate_construct_target(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // collect the written generic arguments in order
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
                let error = self.intern_type(dir::Type::Error)?;

                return Ok(error);
            }
            let arguments = self.intern_type_ids(&[])?;
            let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol,
                arguments,
            }))?;

            return Ok(target);
        };

        // infer omitted construct arguments
        let parameters = self.generic_template_parameters(template)?;
        let Some(substitution) = self.instantiate_parameters(
            origin,
            &parameters,
            &written,
            TypeSubstitution::default(),
            TypeArgumentInference::Exact,
        )?
        else {
            let name = self.format_symbol(symbol);
            let expected = self.writable_parameter_count(&parameters);
            self.report_wrong_generic_arity(module, source.local_id, name, expected, written.len());
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(error);
        };

        // register every bound and predicate on the constructed application
        for constraint in
            self.substitute_application_constraints(origin, template, &substitution)?
        {
            self.check.push_constraint(constraint)?;
        }

        // apply the instantiated arguments to the nominal head
        let arguments = substitution.arguments().collect::<SmallVec<[_; 4]>>();
        let arguments = self.intern_type_ids(&arguments)?;
        let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;

        Ok(target)
    }

    /// Select the construction meaning of one new expression.
    pub(in crate::check) fn select_construct(
        &mut self,
        site: FlowSite,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        expectation: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // collect the supplied arguments once for every candidate
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // separate destination forms from the constructed instance
        let mut forms = SmallVec::<[dir::Form; 2]>::new();
        let mut expected_value = expectation.map(|expectation| expectation.target);
        while let Some(expected) = expected_value {
            let head = self.reduce_type_head(origin, expected)?;
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
        let target = self.select_construct_target(site, ty, expected_value)?;
        let target = self.reduce_type_head(origin, target)?;

        // construct erased interface values through their apparent signatures
        if let dir::Type::Dynamic(dynamic) = self.ty(target)? {
            return self.select_dynamic_construct(
                site,
                node,
                origin,
                target,
                dynamic.constraint,
                argument_nodes,
                &arguments,
                &forms,
            );
        }

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
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    let error = self.commit_error_node(node)?;

                    return Ok(error);
                }

                let constructors = definition.constructors.clone();
                let extends = definition.extends.clone();
                let mut active = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
                self.collect_class_construct_candidates(
                    origin,
                    target,
                    &instance,
                    constructors,
                    extends,
                    &mut active,
                )?
            }
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };
        if constructors.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        }

        // resolve relative constructor member types through the destination place
        let receiver = forms
            .iter()
            .find_map(|form| match form {
                dir::Form::Placed { place } => Some(*place),
                _ => None,
            })
            .map(|place| {
                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value: target,
                }))
            })
            .transpose()?;

        // expect the constructed instance in its destination place
        let expectation = receiver.or(expected_value).and_then(|target| {
            expectation.map(|expectation| Expectation {
                target,
                ..expectation
            })
        });

        // select the first applicable constructor in declaration order
        let is_single_candidate = constructors.len() == 1;
        let mut selected = None;
        let mut rejections = Vec::new();
        for constructor in constructors {
            if is_single_candidate {
                selected = Some(constructor);
                break;
            }
            let (verdict, rejection) = self.probe_candidate_describing(
                |state| {
                    let outcome = state.attempt_construct(
                        origin,
                        target.module_id,
                        &instance,
                        target,
                        constructor.ty,
                        &arguments,
                        expectation,
                        receiver,
                    )?;
                    Ok(outcome.into_candidate())
                },
                |state, rejection| {
                    state
                        .check
                        .describe_signature_rejection(module, constructor.ty, rejection)
                },
            )?;
            match verdict {
                CandidateVerdict::Rejected => rejections.extend(rejection),
                CandidateVerdict::Viable | CandidateVerdict::Indeterminate => {
                    selected = Some(constructor);

                    break;
                }
            }
        }

        // confirm the selected declaration outside any probe
        if let Some(constructor) = selected {
            let attempt = self.attempt_construct(
                origin,
                target.module_id,
                &instance,
                target,
                constructor.ty,
                &arguments,
                expectation,
                receiver,
            )?;

            match attempt {
                SignatureMatch::Selected(signature) | SignatureMatch::ReturnMismatch(signature) => {
                    return self.commit_construct(
                        node,
                        module,
                        target.module_id,
                        argument_nodes,
                        &instance,
                        constructor.constructor,
                        signature,
                        &forms,
                    );
                }
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                } if is_single_candidate => {
                    self.report_signature_rejection(origin, rejection)?;
                    let produced = self.commit_construct(
                        node,
                        module,
                        target.module_id,
                        argument_nodes,
                        &instance,
                        constructor.constructor,
                        selection,
                        &forms,
                    )?;

                    return Ok(produced);
                }
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    self.report_signature_rejection(origin, rejection)?;
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    let error = self.commit_error_node(node)?;

                    return Ok(error);
                }
                SignatureMatch::Invalid { .. } => {}
                SignatureMatch::Inapplicable(_) => {}
            }
        }

        rejections.truncate(4);
        self.reject_construct(site, node, origin, argument_nodes, &rejections)
    }

    /// Return construct candidates for one class instance.
    pub(in crate::check) fn collect_class_construct_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericApplication,
        constructors: Vec<dir::ClassConstructorDefinition>,
        extends: Option<dir::NominalHeritage>,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        if !constructors.is_empty() {
            return Ok(constructors);
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
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        let (extends_module, instance) = self.nominal_application(extends.ty)?;
        if active.contains(&instance.symbol) {
            return Ok(Vec::new());
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
        let arguments = self.intern_type_ids(&arguments)?;
        let instance = dir::GenericApplication {
            arguments,
            ..instance
        };
        let base_receiver = self.intern_type(dir::Type::Application(instance))?;
        let base_constructors = self.collect_class_construct_candidates(
            origin,
            base_receiver,
            &instance,
            base.constructors,
            base.extends,
            active,
        )?;
        active.pop();

        let substitution = self
            .instance_substitution(module, &instance)?
            .with_receiver(receiver);
        let mut constructors = Vec::with_capacity(base_constructors.len());
        for base_constructor in base_constructors {
            let constructor = base_constructor.constructor.forwarded(instance.symbol);
            let ty = self.substitute_type(base_constructor.ty, &substitution)?;
            let ty = self.class_constructor_returning(origin, ty, receiver)?;

            constructors.push(dir::ClassConstructorDefinition { constructor, ty });
        }

        Ok(constructors)
    }

    /// Return one constructor signature with a replaced return type.
    fn class_constructor_returning(
        &mut self,
        _origin: Origin,
        ty: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(mut function) = self.signature_head(ty)? else {
            return Err(CompilerError::Internal {
                message: format!("class constructor type {ty:?} is not a function signature"),
            });
        };
        function.return_type = Some(receiver);

        self.intern_signature(function)
    }

    /// Attempt one constructor candidate against collected arguments.
    pub(in crate::check) fn attempt_construct(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
        function_type: dir::GlobalTypeId,
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<SignatureMatch> {
        // reduce the constructor shape before matching arguments
        let function_type = self.reduce_type_head(origin, function_type)?;
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol)?;
        if instance.arguments.is_empty() && template.is_some() {
            return self.match_signature(
                origin,
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
        let function_type = self.substitute_type(function_type, &substitution)?;
        let function_type = self.reduce_type_head(origin, function_type)?;
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };

        let return_type = function.return_type.or(Some(target));
        let carried = self.settled_argument_bindings(&substitution.bindings)?;
        self.match_signature(
            origin,
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
    ) -> CompilerResult<ValueCheck> {
        let matched = self.match_newtype(
            origin,
            symbol,
            argument_nodes,
            type_arguments,
            expectation,
            NewtypeOverload::Ordered,
            ValueUse::Argument,
        )?;
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
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    let source = self.commit_error_node(node)?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(ValueCheck {
                        source,
                        outcome: CheckOutcome::Fails(CheckFailure::Relation),
                        target,
                    });
                }
                NewtypeRejection::NoMatch(notes) => {
                    let source =
                        self.reject_construct(site, node, origin, argument_nodes, &notes)?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(ValueCheck {
                        source,
                        outcome: CheckOutcome::Fails(CheckFailure::Relation),
                        target,
                    });
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
        if let Some(rejection) = rejection {
            self.report_signature_rejection(origin, rejection)?;
        }

        self.commit_newtype_construct(
            node,
            origin,
            argument_nodes,
            signature,
            expectation,
            outcome,
        )
    }

    /// Commit one selected newtype construction at its call site.
    fn commit_newtype_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: NewtypeSignature,
        expectation: Option<Expectation>,
        outcome: CheckOutcome,
    ) -> CompilerResult<ValueCheck> {
        let module = origin.module();
        let NewtypeSignature {
            selection,
            signature,
        } = signature;
        // commit conversions only after the backing has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }
        let target = dir::ConstructTarget::Newtype(selection);
        let resolution = dir::ConstructDecision::new(
            target,
            self.selected_argument_bindings(node, module, argument_nodes, &signature)?,
            signature.return_type,
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;

        let expected = expectation.map_or(signature.return_type, |expectation| expectation.target);

        Ok(ValueCheck {
            source: signature.return_type,
            outcome,
            target: expected,
        })
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
        forms: &[dir::Form],
    ) -> CompilerResult<dir::GlobalTypeId> {
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
        let mut produced = signature.return_type;
        for form in forms.iter().rev().copied() {
            produced = self.intern_type(dir::Type::Form(dir::FormType {
                form,
                value: produced,
            }))?;
        }
        let resolution = dir::ConstructDecision::new(
            target,
            self.selected_argument_bindings(node, module, argument_nodes, &signature)?,
            produced,
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;

        self.commit_node_type(node, produced)?;

        Ok(produced)
    }

    /// Select one construction through an erased interface construct signature.
    fn select_dynamic_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        arguments: &[CallableArgument],
        forms: &[dir::Form],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = node.module_id;
        let signatures =
            self.apparent_signatures(origin, constraint, SignatureFamily::Construct)?;
        let Some((constraint_module, instance)) = self.nominal_application_maybe(constraint)?
        else {
            return self.reject_not_constructible(node, origin, target, "");
        };
        if signatures.is_empty() {
            return self.reject_not_constructible(node, origin, target, "");
        }

        // select the first applicable construct signature in declaration order
        let is_single_candidate = signatures.len() == 1;
        let mut selected = None;
        let mut rejections = Vec::new();
        for signature in signatures {
            if is_single_candidate {
                selected = Some(signature);
                break;
            }
            let (verdict, rejection) = self.probe_candidate_describing(
                |state| {
                    let outcome = state.attempt_construct(
                        origin,
                        constraint_module,
                        &instance,
                        target,
                        signature.ty,
                        arguments,
                        None,
                        None,
                    )?;
                    Ok(outcome.into_candidate())
                },
                |state, rejection| {
                    state
                        .check
                        .describe_signature_rejection(module, signature.ty, rejection)
                },
            )?;
            match verdict {
                CandidateVerdict::Rejected => rejections.extend(rejection),
                CandidateVerdict::Viable | CandidateVerdict::Indeterminate => {
                    selected = Some(signature);

                    break;
                }
            }
        }

        // confirm the selected signature outside any probe
        if let Some(signature) = selected {
            let attempt = self.attempt_construct(
                origin,
                constraint_module,
                &instance,
                target,
                signature.ty,
                arguments,
                None,
                None,
            )?;

            match attempt {
                SignatureMatch::Selected(selection) | SignatureMatch::ReturnMismatch(selection) => {
                    return self.commit_dynamic_construct(
                        node,
                        module,
                        target,
                        constraint,
                        signature.source,
                        argument_nodes,
                        selection,
                        forms,
                    );
                }
                // report a lone signature's rejection but keep its committed shape
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                } if is_single_candidate => {
                    self.report_signature_rejection(origin, rejection)?;

                    return self.commit_dynamic_construct(
                        node,
                        module,
                        target,
                        constraint,
                        signature.source,
                        argument_nodes,
                        selection,
                        forms,
                    );
                }
                SignatureMatch::Invalid { rejection, .. } => {
                    let description =
                        self.describe_signature_rejection(module, signature.ty, &rejection)?;
                    rejections.push(description);
                }
                SignatureMatch::Inapplicable(rejection) => {
                    let description =
                        self.describe_signature_rejection(module, signature.ty, &rejection)?;
                    rejections.push(description);
                }
            }
        }

        self.reject_construct(site, node, origin, argument_nodes, &rejections)
    }

    /// Commit one selected dynamic construction.
    fn commit_dynamic_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        target: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: SignatureSelection,
        forms: &[dir::Form],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // commit conversions only after the signature has been selected
        for (argument, coercion) in &signature.coercions {
            self.commit_coercion(*argument, coercion.clone())?;
        }

        // dispatch through the callee value's own construct slot
        let construct_target = dir::ConstructTarget::Dynamic {
            dispatch: dir::DynamicDispatch {
                receiver: dir::AdjustedReceiver::direct(target),
                constraint,
            },
            function: dir::DynamicFunction::ConstructSignature(source),
        };

        self.commit_construct_decision(
            node,
            module,
            construct_target,
            argument_nodes,
            &signature,
            forms,
        )
    }

    /// Select the base class constructor initialized by one super call.
    pub(in crate::check) fn select_super_construct(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<ValueCheck> {
        let node = site.node;
        let module = node.module_id;
        let origin = site.origin();
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // type the super callee at its first visit
        let callee_site = self.visit_site(callee.into_global_any(module))?;
        self.infer_node_type(callee_site, PlaceUse::Read)?;

        // read the base instance committed on the super callee
        let super_ty = self.require_node_type(callee.into_global_any(module))?;
        let super_ty = self.reduce_type_head(origin, super_ty)?;
        // poison the call when the super type already reported an error
        if matches!(self.ty(super_ty)?, dir::Type::Error) {
            return self.poison_call(node, None);
        }
        let (base_module, instance) = self.nominal_application(super_ty)?;
        let Some(dir::Definition::Class(base)) = self.definition(instance.symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("super target {:?} has no class definition", instance.symbol),
            });
        };
        let base = base.clone();

        // collect base constructors including forwarded defaults
        let mut active = SmallVec::new();
        let constructors = self.collect_class_construct_candidates(
            origin,
            super_ty,
            &instance,
            base.constructors,
            base.extends,
            &mut active,
        )?;

        // select the first applicable base constructor in declaration order
        let is_single_candidate = constructors.len() == 1;
        let mut selected = None;
        let mut rejections = Vec::new();
        for constructor in constructors {
            if is_single_candidate {
                selected = Some(constructor);
                break;
            }
            let (verdict, rejection) = self.probe_candidate_describing(
                |state| {
                    let outcome = state.attempt_construct(
                        origin,
                        base_module,
                        &instance,
                        super_ty,
                        constructor.ty,
                        &arguments,
                        None,
                        None,
                    )?;
                    Ok(outcome.into_candidate())
                },
                |state, rejection| {
                    state
                        .check
                        .describe_signature_rejection(module, constructor.ty, rejection)
                },
            )?;
            match verdict {
                CandidateVerdict::Rejected => rejections.extend(rejection),
                CandidateVerdict::Viable | CandidateVerdict::Indeterminate => {
                    selected = Some(constructor);

                    break;
                }
            }
        }

        // confirm the selected constructor outside any probe
        if let Some(constructor) = selected {
            let attempt = self.attempt_construct(
                origin,
                base_module,
                &instance,
                super_ty,
                constructor.ty,
                &arguments,
                None,
                None,
            )?;

            match attempt {
                SignatureMatch::Selected(signature)
                | SignatureMatch::ReturnMismatch(signature)
                | SignatureMatch::Invalid {
                    selection: signature,
                    ..
                } => {
                    return self.commit_super_construct(
                        node,
                        module,
                        base_module,
                        &instance,
                        constructor.constructor,
                        argument_nodes,
                        signature,
                    );
                }
                SignatureMatch::Inapplicable(rejection) => {
                    let description =
                        self.describe_signature_rejection(module, constructor.ty, &rejection)?;
                    rejections.push(description);
                }
            }
        }

        // reject the super call when no base constructor accepts the arguments
        rejections.truncate(4);
        self.reject_construct(site, node, origin, argument_nodes, &rejections)?;

        self.reject_call(node, None)
    }

    /// Commit one selected base constructor as the super initialization.
    fn commit_super_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        base_module: ModuleId,
        instance: &dir::GenericApplication,
        constructor: dir::ClassConstructor,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: SignatureSelection,
    ) -> CompilerResult<ValueCheck> {
        // commit conversions only after the constructor has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        // bind generic arguments from the base instance when inference stayed closed
        let generic_arguments = if signature.generic_arguments.is_empty() {
            let arguments = self.type_ids(base_module, instance.arguments)?.to_vec();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };

        // a super call initializes this and produces no value
        let produced = self.intern_type(dir::Type::Void)?;
        let target = dir::ConstructTarget::Class(dir::ClassConstructCandidate {
            symbol: instance.symbol,
            constructor,
            generic_arguments,
        });
        let resolution = dir::ConstructDecision::new(
            target,
            self.selected_argument_bindings(node, module, argument_nodes, &signature)?,
            produced,
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;
        self.commit_node_type(node, produced)?;

        Ok(ValueCheck {
            source: produced,
            outcome: CheckOutcome::Holds,
            target: produced,
        })
    }

    /// Commit one selected construct target with its produced instance type.
    fn commit_construct_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        target: dir::ConstructTarget,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: &SignatureSelection,
        forms: &[dir::Form],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // wrap the produced instance in its destination forms
        let mut produced = signature.return_type;
        for form in forms.iter().rev().copied() {
            produced = self.intern_type(dir::Type::Form(dir::FormType {
                form,
                value: produced,
            }))?;
        }

        // bind the arguments and commit the selection
        let resolution = dir::ConstructDecision::new(
            target,
            self.selected_argument_bindings(node, module, argument_nodes, signature)?,
            produced,
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;
        self.commit_node_type(node, produced)?;

        Ok(produced)
    }

    /// Reject one construction whose arguments fit no constructor.
    pub(in crate::check) fn reject_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        rejections: &[String],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let arguments = self.infer_argument_types(site, argument_nodes)?;
        self.report_no_matching_construct(origin, &arguments, rejections)?;
        self.commit_decision(node, dir::Decision::Rejected)?;
        let error = self.commit_error_node(node)?;

        Ok(error)
    }

    /// Reject one construction whose target cannot use new.
    fn reject_not_constructible(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
        hint: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.report_not_constructible(origin, target, hint)?;
        self.commit_decision(node, dir::Decision::Rejected)?;
        let error = self.commit_error_node(node)?;

        Ok(error)
    }
}
