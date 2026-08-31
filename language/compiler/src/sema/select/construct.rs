use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    CallableArgument, CheckFailure, CheckOutcome, CheckState, Expectation, FlowSite, NewtypeMatch,
    NewtypeSignature, Origin, OverloadRule, OverloadSelection, PlaceUse, SignatureFamily,
    SignatureMatch, SignatureRejection, SignatureSelection, TypeArgumentInference,
    TypeSubstitution, ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the target type named by one construct head.
    pub(in crate::sema) fn select_construct_target(
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
        if let Some(target) = self.own_node_type(source) {
            return Ok(target);
        }

        // let the expected target decide omitted heads
        if matches!(
            self.module(module).view().get(ty),
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                ..
            }
        ) {
            let Some(expected) = self.expected_construct_target(expected)? else {
                self.report_cannot_infer_node(site.node)?;
                let error = self.intern_type(dir::Type::Error)?;

                return Ok(error);
            };
            self.commit_node_type(source, expected)?;

            return Ok(expected);
        }

        // supply omitted generic arguments from a uniquely matching contextual arm
        if let dir::TypeExpression::Reference {
            generic_arguments, ..
        } = self.module(module).view().get(ty)
            && generic_arguments.is_empty()
            && let Some(expected) = expected
            && let Some(resolution) = self.resolutions(source.module_id).name_resolution(source)
            && let [symbol] = resolution.symbols()
            && let Some(expected) = self.expected_construct_instance(expected, *symbol)?
        {
            self.commit_node_type(source, expected)?;

            return Ok(expected);
        }

        self.construct_head_type(origin, module, ty)
    }

    /// Return the nominal head type one construct or pattern writes.
    pub(in crate::sema) fn construct_head_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = ty.into_global_any(module);

        // return the committed construct head
        if let Some(target) = self.own_node_type(source) {
            return Ok(target);
        }

        // keep strict annotation typing for every other head
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
                    self.report_ambiguous_reference(module, ty.into_any(), &path)?;
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
                // accept an undecided head in a module that already reported errors
                None if !self.module(module).diagnostics.is_empty() => {
                    let error = self.intern_type(dir::Type::Error)?;

                    return Ok(error);
                }
                // reference heads in a clean module decide during the walk
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
        if self.symbol_kind(symbol)?.is_binding() {
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
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(expected) = expected else {
            return Ok(None);
        };

        // peel the owned and managed forms around the constructed instance
        let target = match self.ty(expected)? {
            dir::Type::Form(form)
                if matches!(form.form, dir::Form::Owned | dir::Form::Managed { .. }) =>
            {
                form.value
            }
            _ => expected,
        };

        Ok(Some(target))
    }

    /// Return the unique contextual instance of one construct declaration.
    fn expected_construct_instance(
        &mut self,
        expected: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(&[expected]);
        let mut matched = None;

        // search direct, owned, and union targets for one matching nominal head
        while let Some(candidate) = pending.pop() {
            match self.ty(candidate)? {
                dir::Type::Application(instance) if instance.symbol == symbol => {
                    if matched.is_some() {
                        return Ok(None);
                    }
                    matched = Some(candidate);
                }
                dir::Type::Form(form)
                    if matches!(form.form, dir::Form::Owned | dir::Form::Managed { .. }) =>
                {
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

        // push every bound and predicate on the constructed application
        for constraint in
            self.substitute_application_constraints(origin, template, &substitution)?
        {
            self.push_relation(constraint)?;
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
    pub(in crate::sema) fn select_construct(
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

        // walk the argument decorators, keeping the statically present arguments
        let argument_nodes = self.walk_body_arguments(module, argument_nodes)?;
        let argument_nodes = argument_nodes.as_slice();

        // collect the supplied arguments once for every candidate
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // separate destination forms from the constructed instance
        let mut forms = SmallVec::<[dir::Form; 2]>::new();
        let mut expected_value = expectation.map(|expectation| expectation.target);
        while let Some(expected) = expected_value {
            let dir::Type::Form(form) = self.ty(expected)? else {
                break;
            };
            if !matches!(form.form, dir::Form::Owned | dir::Form::Managed { .. }) {
                break;
            }
            forms.push(form.form);
            expected_value = Some(form.value);
        }

        // select the constructed target
        let target = self.select_construct_target(site, ty, expected_value)?;

        // construct erased values through their apparent constraint signatures
        if let Some(constraint) = self.erased_constraint(target)? {
            return self.select_dynamic_construct(
                site,
                node,
                origin,
                target,
                constraint,
                argument_nodes,
                &arguments,
                &forms,
            );
        }

        // read the nominal instance the target names
        let instance = match self.ty(target)? {
            dir::Type::Application(instance) => instance,
            _ => return self.report_rejected_construct_target(node, origin, target, "'new'", ""),
        };

        // require a class to construct through new
        let constructors = match self.definition(instance.symbol)? {
            Some(dir::Definition::Struct(_)) => {
                return self.report_rejected_construct_target(
                    node,
                    origin,
                    target,
                    "'new'",
                    "; construct value types with 'T { … }'",
                );
            }
            Some(dir::Definition::Newtype(_)) => {
                return self.report_rejected_construct_target(
                    node,
                    origin,
                    target,
                    "'new'",
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
            _ => return self.report_rejected_construct_target(node, origin, target, "'new'", ""),
        };

        // require at least one construct candidate
        if constructors.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        }

        // resolve relative constructor member types through the destination place
        let receiver = forms
            .iter()
            .find_map(|form| match form {
                dir::Form::Managed { place } => Some(*place),
                _ => None,
            })
            .map(|place| {
                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed { place },
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
        let selection = self.select_callable(
            origin,
            &constructors,
            OverloadRule::Ordered,
            |constructor| constructor.ty,
            |state, constructor| {
                state.match_construct(
                    origin,
                    target.module_id,
                    &instance,
                    target,
                    constructor.ty,
                    &arguments,
                    expectation,
                    receiver,
                )
            },
        )?;

        // commit or report the constructor the match selected
        match selection {
            // commit the selected construction under the constructor's declared visibility
            OverloadSelection::Selected {
                candidate: constructor,
                signature,
                ..
            } => {
                if let Some(symbol) = constructor.constructor.call_symbol() {
                    self.check_symbol_access(origin, symbol, "constructor")?;
                }

                self.commit_construct(
                    node,
                    module,
                    target.module_id,
                    argument_nodes,
                    &instance,
                    constructor.constructor.clone(),
                    signature,
                    &forms,
                )
            }
            // commit the rejection a sole candidate already reported
            OverloadSelection::Refused => {
                self.commit_decision(node, dir::Decision::Rejected)?;

                self.commit_error_node(node)
            }
            // report the arguments every candidate rejected
            OverloadSelection::Rejected(rejections) => {
                self.report_rejected_construct(site, node, origin, argument_nodes, &rejections)
            }
            // fail on an ambiguity an ordered rule settles
            OverloadSelection::Ambiguous => Err(CompilerError::Internal {
                message: "ordered construct selection reported an ambiguous constructor"
                    .to_string(),
            }),
        }
    }

    /// Return construct candidates for one class instance.
    pub(in crate::sema) fn collect_class_construct_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericApplication,
        constructors: Vec<dir::ClassConstructorDefinition>,
        extends: Option<dir::NominalHeritage>,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        // keep the constructors the class declares itself
        if !constructors.is_empty() {
            return Ok(constructors);
        }

        // require a base class to forward from
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
        // stop at a heritage cycle and mark this base active
        let (extends_module, instance) = self.nominal_application(extends.ty)?;
        if active.contains(&instance.symbol) {
            return Ok(Vec::new());
        }
        active.push(instance.symbol);

        // read the base class definition
        let base = match self.definition(instance.symbol)? {
            Some(dir::Definition::Class(base)) => base.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("base class {:?} has no checked definition", instance.symbol),
                });
            }
        };

        // apply the written heritage arguments to the base instance
        let module = origin.module();
        let arguments: SmallVec<[_; 8]> = self.type_ids(extends_module, instance.arguments)?.into();
        let arguments = self.intern_type_ids(&arguments)?;
        let instance = dir::GenericApplication {
            arguments,
            ..instance
        };
        let base_receiver = self.intern_type(dir::Type::Application(instance))?;

        // collect the constructors the base itself offers
        let base_constructors = self.collect_class_construct_candidates(
            origin,
            base_receiver,
            &instance,
            base.constructors,
            base.extends,
            active,
        )?;
        active.pop();

        // forward each base constructor onto the derived receiver
        let substitution = self
            .instance_substitution(module, &instance)?
            .with_receiver(receiver);
        let mut constructors = Vec::with_capacity(base_constructors.len());
        for base_constructor in base_constructors {
            let constructor = base_constructor.constructor.forwarded(instance.symbol);
            let ty = self.substitute_type(base_constructor.ty, &substitution)?;
            let ty = self.class_constructor_returning(ty, receiver)?;

            constructors.push(dir::ClassConstructorDefinition { constructor, ty });
        }

        Ok(constructors)
    }

    /// Return one constructor signature with a replaced return type.
    fn class_constructor_returning(
        &mut self,
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

    /// Match one constructor candidate against collected arguments.
    pub(in crate::sema) fn match_construct(
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
        // read the constructor shape before matching arguments
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol)?;
        let infers_arguments = match template {
            Some(template) if instance.arguments.is_empty() => {
                let parameters = self.generic_template_parameters(template)?;

                self.writable_parameter_count(&parameters) != 0
            }
            _ => false,
        };
        if infers_arguments {
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

        // substitute the written arguments of an applied class
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(target);
        let function_type = self.substitute_type(function_type, &substitution)?;
        let Some(function) = self.signature_head(function_type)? else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };

        // fall back to the constructed target as the return type
        let return_type = function.return_type.or(Some(target));
        let carried = self.resolved_argument_bindings(&substitution.bindings)?;

        // match the constructor signature against the written arguments
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
    pub(in crate::sema) fn select_newtype_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        // match the written arguments against the newtype's backings
        let matched = self.match_newtype(
            origin,
            symbol,
            argument_nodes,
            type_arguments,
            expectation,
            OverloadRule::Ordered,
            ValueUse::Argument,
        )?;

        // commit the selected backing, or fail the rejected construction
        let (signature, outcome) = match matched {
            NewtypeMatch::Selected(signature, outcome) => (signature, outcome),
            NewtypeMatch::Refused => {
                self.commit_decision(node, dir::Decision::Rejected)?;
                let source = self.commit_error_node(node)?;
                let target = expectation.map_or(source, |expectation| expectation.target);

                return Ok(ValueCheck {
                    source,
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                });
            }
            NewtypeMatch::Rejected(notes) => {
                let source =
                    self.report_rejected_construct(site, node, origin, argument_nodes, &notes)?;
                let target = expectation.map_or(source, |expectation| expectation.target);

                return Ok(ValueCheck {
                    source,
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                });
            }
            NewtypeMatch::Ambiguous => {
                return Err(CompilerError::Internal {
                    message: "ordered newtype selection reported an ambiguous backing".to_string(),
                });
            }
        };

        // commit the selected newtype construction
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
            key,
            backing,
            signature,
        } = signature;

        // deny a construction the backing's declared visibility rejects
        self.check_backing_access(origin, key.symbol)?;

        // commit conversions only after the backing has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        // commit the construction over the selected newtype backing
        let target = dir::ConstructTarget::Newtype { key, backing };
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
        // bind generic arguments from the class instance when inference stayed closed
        let generic_arguments = if signature.generic_arguments.is_empty() {
            let arguments: SmallVec<[_; 8]> =
                self.type_ids(instance_module, instance.arguments)?.into();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };

        // commit conversions only after the constructor has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        // select the class and the constructor this construction runs
        let target = dir::ConstructTarget::Class {
            key: dir::InstanceKey::new(instance.symbol, generic_arguments),
            constructor,
        };

        // wrap the produced instance in the destination forms, replacing its own heap form
        let mut produced = signature.return_type;
        if !forms.is_empty()
            && let dir::Type::Form(form) = self.ty(produced)?
            && matches!(form.form, dir::Form::Managed { .. })
        {
            produced = form.value;
        }
        for form in forms.iter().rev().copied() {
            produced = self.intern_type(dir::Type::Form(dir::FormType {
                form,
                value: produced,
            }))?;
        }

        // commit the construction over the selected class constructor
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
        // read the construct signatures the constraint declares
        let module = node.module_id;
        let signatures = self.apparent_signatures(constraint, SignatureFamily::Construct)?;
        let Some((constraint_module, instance)) = self.nominal_application_maybe(constraint)?
        else {
            return self.report_rejected_construct_target(node, origin, target, "'new'", "");
        };
        if signatures.is_empty() {
            return self.report_rejected_construct_target(node, origin, target, "'new'", "");
        }

        // select the first applicable construct signature in declaration order
        let selection = self.select_callable(
            origin,
            &signatures,
            OverloadRule::Ordered,
            |signature| signature.ty,
            |state, signature| {
                state.match_construct(
                    origin,
                    constraint_module,
                    &instance,
                    target,
                    signature.ty,
                    arguments,
                    None,
                    None,
                )
            },
        )?;

        // commit or report the construct signature the match selected
        match selection {
            // commit the dynamic construction the signature match selected
            OverloadSelection::Selected {
                candidate,
                signature,
                ..
            } => self.commit_dynamic_construct(
                node,
                module,
                target,
                constraint,
                candidate.source,
                argument_nodes,
                signature,
                forms,
            ),
            // commit the rejection a sole candidate already reported
            OverloadSelection::Refused => {
                self.commit_decision(node, dir::Decision::Rejected)?;

                self.commit_error_node(node)
            }
            // report the arguments every candidate rejected
            OverloadSelection::Rejected(rejections) => {
                self.report_rejected_construct(site, node, origin, argument_nodes, &rejections)
            }
            // fail on an ambiguity an ordered rule settles
            OverloadSelection::Ambiguous => Err(CompilerError::Internal {
                message: "ordered construct selection reported an ambiguous signature".to_string(),
            }),
        }
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

        // dispatch through the callee value's own construct signature
        let construct_target = dir::ConstructTarget::Dynamic {
            dispatch: dir::DynamicDispatch {
                receiver: dir::AdjustedReceiver::direct(target),
                constraint,
            },
            function: dir::DynamicFunction::ConstructSignature(source),
        };

        // commit the erased construct decision
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
    pub(in crate::sema) fn select_super_construct(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<ValueCheck> {
        // collect the supplied arguments once for every candidate
        let node = site.node;
        let module = node.module_id;
        let origin = site.origin();
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // type the super callee at its first visit
        let callee_site = self.visit_site(callee.into_global_any(module))?;
        self.infer_node_type(callee_site, PlaceUse::Read)?;

        // read the base instance committed on the super callee
        let super_ty = self.require_node_type(callee.into_global_any(module))?;

        // poison the call when the super type carries a reported error
        if matches!(self.ty(super_ty)?, dir::Type::Error) {
            return self.poison_call(node, None);
        }

        // read the base class this super call initializes
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
        let selection = self.select_callable(
            origin,
            &constructors,
            OverloadRule::Ordered,
            |constructor| constructor.ty,
            |state, constructor| {
                state.match_construct(
                    origin,
                    base_module,
                    &instance,
                    super_ty,
                    constructor.ty,
                    &arguments,
                    None,
                    None,
                )
            },
        )?;

        // commit or report the constructor the match selected
        match selection {
            // commit the base constructor the match selected
            OverloadSelection::Selected {
                candidate: constructor,
                signature,
                ..
            } => self.commit_super_construct(
                node,
                module,
                base_module,
                &instance,
                constructor.constructor.clone(),
                argument_nodes,
                signature,
            ),
            // commit the rejection a sole candidate already reported
            OverloadSelection::Refused => self.commit_rejected_call(node, None, None),
            // report the arguments every candidate rejected
            OverloadSelection::Rejected(rejections) => {
                self.report_rejected_construct(site, node, origin, argument_nodes, &rejections)?;

                self.commit_rejected_call(node, None, None)
            }
            // fail on an ambiguity an ordered rule settles
            OverloadSelection::Ambiguous => Err(CompilerError::Internal {
                message: "ordered super selection reported an ambiguous constructor".to_string(),
            }),
        }
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
            let arguments: SmallVec<[_; 8]> =
                self.type_ids(base_module, instance.arguments)?.into();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };

        // initialize this through a super call, which produces void
        let produced = self.intern_type(dir::Type::Void)?;
        let target = dir::ConstructTarget::Class {
            key: dir::InstanceKey::new(instance.symbol, generic_arguments),
            constructor,
        };
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

    /// Report one construction that every candidate constructor rejected.
    pub(in crate::sema) fn report_rejected_construct(
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

    /// Reject one aggregate construction head outside the value families.
    pub(in crate::sema) fn require_aggregate_construct_target(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the nominal head the aggregate names
        let symbol = match self.ty(target)? {
            dir::Type::Application(instance) => instance.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(None),
        };

        // reject the families aggregate literals never construct
        let hint = match self.definition(symbol)? {
            Some(dir::Definition::Class(_)) => "; construct classes with 'new T(\u{2026})'",
            Some(dir::Definition::Enum(_)) => "; construct enum values through their variants",
            Some(dir::Definition::Newtype(_)) => "; construct newtypes with 'T(\u{2026})'",
            Some(dir::Definition::Interface(_)) => "",
            _ => return Ok(None),
        };
        let error =
            self.report_rejected_construct_target(node, origin, target, "'T { \u{2026} }'", hint)?;

        Ok(Some(error))
    }

    /// Report one construct target the written form refuses.
    fn report_rejected_construct_target(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
        form: &str,
        hint: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.report_not_constructible(origin, target, form, hint)?;
        self.commit_decision(node, dir::Decision::Rejected)?;
        let error = self.commit_error_node(node)?;

        Ok(error)
    }
}
