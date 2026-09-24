use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    AssignedPlace, CallableArgument, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome,
    CheckState, Expectation, FailedCheck, FlowSite, GenericParameterId, InferMode, MemoryGrounding,
    NewtypeMatch, NewtypeSignature, Origin, OverloadRule, OverloadSelection, PlaceUse, Relation,
    SignatureMatch, SignatureRejection, SignatureSelection, StoreTarget, TypeSubstitution, Value,
    ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Bind a class constructor to its required or declared function signature.
    pub(in crate::sema) fn infer_constructor_value(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // require an allocating constructor for a class value
        if let dir::Type::Reference(reference) = self.ty(source)?
            && let Some(dir::Definition::Class(class)) =
                self.definition(reference.symbol)?.as_deref()
            && class.is_abstract
        {
            let instance = self.declaration_instance(reference.symbol)?;
            let instance = self.intern_type(dir::Type::Application(instance))?;
            self.report_cannot_construct_abstract_type(site.origin(), instance)?;
            self.commit_error_node(site.node)?;

            return Ok(());
        }

        // select a contextual signature before inferring a standalone constructor
        let selected = match context {
            Some(context) => {
                let target = self.strip_form(site.origin(), context.target)?;
                match self.select_constructor_value(site, context.cause, source, target)? {
                    Ok(construction) => construction.map(|construction| (target, construction)),
                    Err(failure) => {
                        self.push_failure(FailedCheck {
                            cause: context.cause,
                            relation: context.relation,
                            use_: Some(context.use_),
                            source,
                            target,
                            failure,
                        })?;
                        self.commit_error_node(site.node)?;

                        return Ok(());
                    }
                }
            }
            None => None,
        };
        let (target, construction) = match selected {
            Some(selected) => selected,
            None => {
                // require one signature when no expected type selects an overload
                let constructors =
                    self.construct_signatures(site.origin(), source, MemoryGrounding::Open)?;
                let [constructor] = constructors.as_slice() else {
                    let dir::Type::Reference(reference) = self.ty(source)? else {
                        return Err(CompilerError::Internal {
                            message: "a constructor value without a class reference".to_string(),
                        });
                    };
                    self.report_ambiguous_overload(site.node, &[reference.symbol])?;
                    self.commit_error_node(site.node)?;

                    return Ok(());
                };
                let target = self.function_type(constructor.ty)?;
                let cause = self.intern_cause(Cause::root(
                    site.origin(),
                    CauseKind::Initializer { annotation: None },
                ));
                let construction = self.select_constructor_value(site, cause, source, target)?;
                let Ok(Some(construction)) = construction else {
                    return Err(CompilerError::Internal {
                        message: "a constructor incompatible with its declared signature"
                            .to_string(),
                    });
                };

                (target, construction)
            }
        };

        // record the allocating entry as an ordinary callable value
        let value = dir::FunctionValue {
            target: dir::CallableTarget::Constructor(Box::new(construction)),
            callable_type: target,
        };
        self.commit_decision(site.node, dir::Decision::Function(value.into()))?;

        self.commit_node_type(site.node, target)
    }

    /// Return one class value's construct signatures, grounding their induced memory parameters.
    pub(in crate::sema) fn construct_signatures(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        grounding: MemoryGrounding,
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        // require a concrete class declaration
        let dir::Type::Reference(reference) = self.ty(source)? else {
            return Ok(Vec::new());
        };
        match self.definition(reference.symbol)?.as_deref() {
            Some(dir::Definition::Class(_)) => {}
            _ => return Ok(Vec::new()),
        }

        // substitute the complete constructor application into its instance and signatures
        let (instance, substitution) = if reference.arguments.is_empty() {
            (
                self.declaration_instance(reference.symbol)?,
                TypeSubstitution::default(),
            )
        } else {
            let instance = dir::GenericApplication {
                symbol: reference.symbol,
                arguments: reference.arguments,
            };
            let substitution = self.instance_substitution(source.module_id, &instance)?;
            let arguments: SmallVec<[_; 4]> =
                self.type_ids(source.module_id, reference.arguments)?.into();
            let arguments = self.intern_type_ids(&arguments)?;

            (
                dir::GenericApplication {
                    arguments,
                    ..instance
                },
                substitution,
            )
        };
        let receiver = self.intern_type(dir::Type::Application(instance))?;
        let template = self.symbol_template(reference.symbol)?;
        let mut constructors =
            self.class_constructors(origin, receiver, &instance, &mut SmallVec::new())?;
        let substitution = substitution.with_receiver(receiver);
        for constructor in &mut constructors {
            let ty = self.substitute_type(constructor.ty, &substitution)?;

            // bind the initializer's induced memory parameters for the allocating constructor
            let head = self
                .signature_head(ty)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "a class constructor without its signature".to_string(),
                })?;
            let parameters = self.signature_generic_parameters(ty.module_id, &head)?;
            let mut induced = SmallVec::<[GenericParameterId; 4]>::new();
            for parameter in parameters {
                let memory = self
                    .require_generic_parameter(parameter)?
                    .induced_memory_parameter();
                // an elided extent stays a binder of the class value's signature
                let is_grounded = match grounding {
                    MemoryGrounding::Open => memory.is_some(),
                    MemoryGrounding::Elided => memory == Some(dir::MemoryParameter::Access),
                };
                if is_grounded {
                    induced.push(parameter);
                }
            }
            let arguments = self.signature_arguments(ty.module_id, head.arguments)?;
            let mut allocation = TypeSubstitution::default()
                .with_carried(arguments)?
                .with_carried(&substitution.bindings)?;
            self.ground_memory_parameters(origin, &induced, &mut allocation, grounding)?;
            let ty = self.substitute_type(ty, &allocation)?;
            let arguments = self.intern_generic_arguments(&allocation.bindings)?;

            // present the declared signature in its construct form, its parameters in this module
            let head = self
                .signature_head(ty)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "a class constructor without its signature".to_string(),
                })?;
            let parameters = self
                .signature_parameters(ty.module_id, head.parameters)?
                .to_vec();
            let parameters = self.intern_parameters(&parameters)?;
            constructor.ty = self.intern_signature(dir::FunctionSignatureType {
                is_construct: true,
                this_parameter: None,
                template: head.template.or(template),
                arguments,
                parameters,
                ..head
            })?;
        }

        Ok(constructors)
    }

    /// Select the construction performed by a required constructor function.
    pub(in crate::sema) fn select_constructor_value(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Result<Option<dir::ConstructDecision>, CheckFailure>> {
        let origin = site.origin();

        // require a class reference converting to a construct signature
        let dir::Type::Reference(_) = self.ty(source)? else {
            return Ok(Ok(None));
        };
        let Some(required) = self.signature_head(target)? else {
            return Ok(Ok(None));
        };
        if !required.is_construct {
            return Ok(Ok(None));
        }

        // select in the same declaration order as a direct construction
        let constructors = self.construct_signatures(origin, source, MemoryGrounding::Open)?;
        for constructor in constructors {
            let Some(instantiation) = self.instantiate_signature(origin, constructor.ty, target)?
            else {
                continue;
            };
            if !self
                .decide_relation(origin, Relation::Subtype, instantiation.signature, target)?
                .holds()
            {
                continue;
            }

            // retain the selected class arguments and constructor result
            let signature = self
                .signature_head(instantiation.signature)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "an instantiated constructor without its signature".to_string(),
                })?;
            let return_type = signature
                .return_type
                .ok_or_else(|| CompilerError::Internal {
                    message: "an instantiated constructor without its result".to_string(),
                })?;
            let returned = self.strip_form(origin, return_type)?;
            let (module, instance) = self.nominal_application(returned)?;
            let arguments: SmallVec<[_; 4]> = self.type_ids(module, instance.arguments)?.into();
            let mut arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &arguments)?;
            let signature_bindings = self
                .signature_arguments(instantiation.signature.module_id, signature.arguments)?
                .to_vec();
            for binding in signature_bindings {
                if !arguments
                    .iter()
                    .any(|bound| bound.parameter == binding.parameter)
                {
                    arguments.push(binding);
                }
            }
            let key = dir::InstanceKey::new(instance.symbol, arguments);

            // supply the generated function's parameters to the selected constructor
            let parameters = self
                .signature_parameters(instantiation.signature.module_id, signature.parameters)?
                .to_vec();
            let supplied = self
                .signature_parameters(target.module_id, required.parameters)?
                .to_vec();
            let Some(mut arguments) = self.forward_parameters(site, &parameters, &supplied)? else {
                continue;
            };
            let supplied = supplied
                .iter()
                .map(|parameter| parameter.ty)
                .collect::<Vec<_>>();
            for argument in &mut arguments {
                if let Err(failure) = self.convert_argument(site, cause, argument, &supplied)? {
                    return Ok(Err(failure));
                }
            }
            if let Some(symbol) = constructor.constructor.call_symbol() {
                self.check_symbol_access(origin, symbol, "constructor")?;
            }
            let target =
                self.class_construct_target(key.symbol, constructor.constructor, key.arguments)?;

            let mut construction =
                dir::ConstructDecision::new(target, arguments, return_type, instantiation.regions);

            // convert the constructed instance to the required function result
            if let Some(required) = required.return_type {
                let value = Value {
                    ty: return_type,
                    node: None,
                    place: None,
                    is_fresh: false,
                };
                construction.coercion = match self.convert_closed_value(
                    site,
                    origin,
                    cause,
                    value,
                    required,
                    ValueUse::Output,
                )? {
                    Ok(coercion) => coercion,
                    Err(failure) => return Ok(Err(failure)),
                };
            }

            return Ok(Ok(Some(construction)));
        }

        Ok(Ok(None))
    }

    /// Select the complete target of an aggregate expression or nominal pattern.
    pub(in crate::sema) fn construct_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // reuse a completed selection
        let source = ty.into_global_any(module);
        if let Some(target) = self.own_node_type(source) {
            return self.normalize(origin, target);
        }

        // resolve written heads and arguments before applying construction context
        self.walk_body_construct_type(module, ty)?;
        if let Some(target) = self.own_node_type(source) {
            return self.normalize(origin, target);
        }
        let expression = self.module(module).view().get(ty).clone();

        // select omitted heads and nominal arguments from the same expected instance
        let target = match expression {
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                ..
            } => match self.expected_construct_target(expected)? {
                Some(target) => target,
                None => {
                    self.report_cannot_infer_node(self.origin_source(origin)?)?;
                    self.intern_type(dir::Type::Error)?
                }
            },
            dir::TypeExpression::Reference {
                generic_arguments, ..
            } => {
                let symbol = self
                    .name_decision(source)
                    .and_then(|resolution| resolution.single_symbol())
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("construct type {source:?} has no declaration"),
                    })?;
                self.instantiate_construct_target(
                    origin,
                    source,
                    symbol,
                    &generic_arguments,
                    expected,
                )?
            }
            _ => return self.require_node_type(source),
        };
        self.commit_node_type(source, target)?;

        Ok(target)
    }

    /// Return the expected target after peeling construction forms.
    fn expected_construct_target(
        &mut self,
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(mut target) = expected else {
            return Ok(None);
        };

        // peel the owned and managed forms around the constructed instance
        while let dir::Type::Form(form) = self.ty(target)?
            && matches!(form.form, dir::Form::Owned)
        {
            target = form.value;
        }

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
                dir::Type::Form(form) if matches!(form.form, dir::Form::Owned) => {
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
        expected: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = source.module_id;

        // infer unwritten arguments from one matching contextual instance
        if arguments.is_empty()
            && let Some(expected) = expected
            && let Some(target) = self.expected_construct_instance(expected, symbol)?
        {
            return Ok(target);
        }

        // read the checked arguments at the construct expression
        let mut written = SmallVec::<[_; 4]>::new();
        for argument in arguments {
            written.push(self.require_node_type(argument.into_global_any(module))?);
        }

        // read the declaration's parameters
        let template = self.symbol_template(symbol)?;
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };

        // infer omitted construct arguments
        let Some(substitution) = self.instantiate_parameters(
            origin,
            &parameters,
            &written,
            TypeSubstitution::default(),
        )?
        else {
            let name = self.format_symbol(symbol);
            let expected = self.writable_parameter_count(&parameters)?;
            self.report_wrong_generic_arity(module, source.local_id, name, expected, written.len());
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(error);
        };

        // push every bound and predicate on the constructed application
        if let Some(template) = template {
            for constraint in
                self.substitute_application_constraints(origin, template, &substitution)?
            {
                self.push_relation(constraint)?;
            }
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
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // walk the argument decorators, keeping the statically present arguments
        let argument_nodes = self.walk_body_arguments(module, argument_nodes)?;
        let argument_nodes = argument_nodes.as_slice();

        // separate destination forms from the constructed instance
        let mut forms = SmallVec::<[dir::Form; 2]>::new();
        let mut expected_value = expectation.and_then(Expectation::contextual_target);
        while let Some(expected) = expected_value {
            let dir::Type::Form(form) = self.ty(expected)? else {
                break;
            };
            if !matches!(form.form, dir::Form::Owned) {
                break;
            }
            forms.push(form.form);
            expected_value = Some(form.value);
        }

        // check the complete constructor value and its written type arguments
        self.walk_body_generic_arguments(module, generic_arguments)?;
        let source = left.into_global_any(module);
        let callee_site = self.visit_site(source)?;
        let is_hole = matches!(
            self.module(module).view().get(left),
            dir::Expression::Infer {
                form: dir::InferForm::Hole,
                name: None
            }
        );
        let target = if is_hole {
            let Some(target) = self.expected_construct_target(expected_value)? else {
                self.report_cannot_infer_node(source)?;
                self.commit_decision(node, dir::Decision::Rejected)?;

                return self.commit_rejected_call(node, expectation, None);
            };
            self.commit_node_type(source, target)?;

            target
        } else {
            // require a value declaration before selecting its constructor
            if let Some(resolution) = self.decide_reference(source)?
                && !self.check_value_reference(source, &resolution)?
            {
                self.commit_error_node(source)?;

                return self.commit_rejected_call(node, expectation, None);
            }

            let (_, value) = self.infer_receiver(callee_site)?;
            let value = self.strip_form(origin, value)?;
            let value = self.normalize(origin, value)?;
            let value = self.static_value_type(value)?;
            match self.ty(value)? {
                // apply the class constructor to its type arguments
                dir::Type::Reference(reference)
                    if self.symbol_kind(reference.symbol)? == dir::SymbolKind::Class =>
                {
                    // require invocation arguments only on an unapplied constructor
                    if !reference.arguments.is_empty() && !generic_arguments.is_empty() {
                        let name = self.format_symbol(reference.symbol);
                        self.report_wrong_generic_arity(
                            module,
                            source.local_id,
                            name,
                            0,
                            generic_arguments.len(),
                        );
                        self.commit_decision(node, dir::Decision::Rejected)?;

                        return self.commit_rejected_call(node, expectation, None);
                    }
                    // preserve checked arguments on a specialized constructor
                    if !reference.arguments.is_empty() {
                        let arguments: SmallVec<[_; 4]> =
                            self.type_ids(value.module_id, reference.arguments)?.into();
                        let arguments = self.intern_type_ids(&arguments)?;
                        self.intern_type(dir::Type::Application(dir::GenericApplication {
                            symbol: reference.symbol,
                            arguments,
                        }))?
                    }
                    // infer an unapplied constructor from its context or written arguments
                    else {
                        self.instantiate_construct_target(
                            origin,
                            source,
                            reference.symbol,
                            generic_arguments,
                            expected_value,
                        )?
                    }
                }
                // retain an already diagnosed operand failure
                dir::Type::Error => {
                    self.commit_decision(node, dir::Decision::Poisoned)?;

                    return self.commit_rejected_call(node, expectation, None);
                }
                // invoke construct signatures through the shared callable selection
                _ => {
                    let check = self.select_construct_call(
                        site,
                        callee_site,
                        value,
                        generic_arguments,
                        argument_nodes,
                        expectation,
                    )?;

                    return Ok(check);
                }
            }
        };

        // collect the supplied arguments once for every nominal constructor
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // read the nominal instance the target names
        let instance = match self.ty(target)? {
            dir::Type::Application(instance) => instance,
            _ => {
                self.report_not_constructible(origin, target, "'new'", "")?;

                return self.commit_rejected_call(node, expectation, None);
            }
        };

        // require a class to construct through new
        let constructors = match self.definition(instance.symbol)?.as_deref() {
            Some(dir::Definition::Struct(_)) => {
                self.report_not_constructible(
                    origin,
                    target,
                    "'new'",
                    "; construct value types with 'T { … }'",
                )?;

                return self.commit_rejected_call(node, expectation, None);
            }
            Some(dir::Definition::Newtype(_)) => {
                self.report_not_constructible(
                    origin,
                    target,
                    "'new'",
                    "; construct newtypes with 'T(…)'",
                )?;

                return self.commit_rejected_call(node, expectation, None);
            }
            Some(dir::Definition::Class(definition)) => {
                if definition.is_abstract {
                    self.report_cannot_construct_abstract_type(origin, target)?;
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    return self.commit_rejected_call(node, expectation, None);
                }

                let mut active = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
                self.class_constructors(origin, target, &instance, &mut active)?
            }
            _ => {
                self.report_not_constructible(origin, target, "'new'", "")?;

                return self.commit_rejected_call(node, expectation, None);
            }
        };

        // require at least one construct candidate
        if constructors.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        }

        // expect the constructed instance where the destination holds it
        let expectation = expected_value.and_then(|target| {
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
                    None,
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

                let source = self.commit_construct(
                    node,
                    target.module_id,
                    &instance,
                    constructor.constructor.clone(),
                    signature,
                    &forms,
                )?;

                Ok(ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target: expectation.map_or(source, |expected| expected.target),
                })
            }
            // commit the rejection a sole candidate already reported
            OverloadSelection::Refused => {
                self.commit_decision(node, dir::Decision::Rejected)?;

                self.commit_rejected_call(node, expectation, None)
            }
            // report the arguments every candidate rejected
            OverloadSelection::Rejected(rejections) => {
                let arguments = self.infer_argument_types(site, argument_nodes)?;
                self.report_no_matching_construct(origin, &arguments, &rejections)?;

                self.commit_rejected_call(node, expectation, None)
            }
            // fail on an ambiguity an ordered rule settles
            OverloadSelection::Ambiguous => Err(CompilerError::Internal {
                message: "ordered construct selection reported an ambiguous constructor"
                    .to_string(),
            }),
        }
    }

    /// Derive and commit one class's construct candidates at its declared parameters.
    pub(in crate::sema) fn derive_class_constructors(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // instantiate the class over its declared parameters
        let template = self.symbol_template(symbol)?;
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let receiver = self.nominal_return_type(symbol, &parameters)?;
        let dir::Type::Application(instance) = self.ty(receiver)? else {
            return Err(CompilerError::Internal {
                message: format!("class {symbol:?} without an application of itself"),
            });
        };

        // record the declared, forwarded, or default candidates
        let origin = Origin::Symbol(symbol);
        let constructors =
            self.class_constructors(origin, receiver, &instance, &mut SmallVec::new())?;
        self.module_mut(symbol.module_id)
            .members_tail
            .set_class_constructors(symbol, constructors);

        Ok(())
    }

    /// Return construct candidates for one class instance.
    pub(in crate::sema) fn class_constructors(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        // read the class declaration at each inheritance step
        let definition = self.definition(instance.symbol)?;
        let Some(dir::Definition::Class(class)) = definition.as_deref() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "constructor target {:?} has no class definition",
                    instance.symbol
                ),
            });
        };

        // keep the constructors the class declares itself
        let mut declared = Vec::new();
        for member in &class.members {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            if method.slot != dir::MemberSlot::Constructor {
                continue;
            }
            declared.push(dir::ClassConstructorDefinition {
                constructor: dir::ClassConstructor::Declared {
                    symbol: method.symbol,
                },
                ty: self.symbol_type(method.symbol)?,
            });
        }
        if !declared.is_empty() {
            return Ok(declared);
        }

        // forward a derived class's base constructors, a base class constructing by `new T()`
        match class.extends.clone() {
            Some(extends) => self.inherited_constructors(origin, receiver, &extends, active),
            None => Ok(vec![self.default_class_constructor(receiver)?]),
        }
    }

    /// Return the `new T()` candidate a base class without a declared constructor offers.
    fn default_class_constructor(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ClassConstructorDefinition> {
        let ty = self.intern_signature(dir::FunctionSignatureType {
            parks: false,
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            arguments: dir::TypeListId::EMPTY,
            this_parameter: None,
            parameters: dir::TypeListId::EMPTY,
            return_type: Some(receiver),
            is_generator: false,
            is_construct: false,
        })?;

        Ok(dir::ClassConstructorDefinition {
            constructor: dir::ClassConstructor::Default,
            ty,
        })
    }

    /// Return constructors forwarded from one base class.
    fn inherited_constructors(
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
        let base_constructors =
            self.class_constructors(origin, base_receiver, &instance, active)?;
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
        // read the constructor shape without its receiver term, which the allocation supplies
        let Some(mut function) = self.signature_head(function_type)? else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };
        let this = function.this_parameter.take();
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol)?;
        let infers_arguments = match template {
            Some(template) if instance.arguments.is_empty() => {
                let parameters = self.generic_template_parameters(template)?;

                self.writable_parameter_count(&parameters)? != 0
            }
            _ => false,
        };
        if infers_arguments {
            let carried =
                self.construct_region_bindings(origin, function_type.module_id, &function, this)?;

            return self.match_signature(
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
            );
        }

        // substitute the written arguments of an applied class
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(target);
        let function_type = self.substitute_type(function_type, &substitution)?;
        let Some(mut function) = self.signature_head(function_type)? else {
            return Ok(SignatureMatch::Inapplicable(
                SignatureRejection::Inapplicable,
            ));
        };
        let this = function.this_parameter.take();

        // construct the target when the constructor writes no result
        let return_type = function.return_type.or(Some(target));
        function.return_type = return_type;

        // pass the class's bindings and the destination place into the constructor's template
        let mut carried = self.resolved_argument_bindings(&substitution.bindings)?;
        carried.extend(self.resolved_region_bindings(&substitution.bindings)?);
        carried.extend(self.construct_region_bindings(
            origin,
            function_type.module_id,
            &function,
            this,
        )?);

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

    /// Bind the constructor's receiver region at the constructed class's space.
    fn construct_region_bindings(
        &mut self,
        origin: Origin,
        module: ModuleId,
        function: &dir::FunctionSignatureType,
        this: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        // bind the region parameter the receiver borrows the construction at
        let mut bound = Vec::new();
        if let Some(this) = this
            && let Some(region) = self.form_chain(origin, this)?.region()
            && let dir::Type::Parameter(region_parameter) = self.ty(self.shallow_resolve(region)?)?
        {
            let place = self.space_term(origin, this)?;
            let extent = self.lifetime_literal(dir::Lifetime::Managed)?;
            let construction = self.intern_region(extent, place)?;
            bound.push((region_parameter, construction));
        }

        let mut bindings = Vec::new();
        for parameter in self.signature_generic_parameters(module, function)? {
            if let Some((_, argument)) = bound.iter().find(|(bound, _)| *bound == parameter) {
                bindings.push(dir::GenericArgumentBinding::new(parameter, *argument));
            }
        }

        Ok(bindings)
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
        self.commit_newtype_construct(node, origin, signature, expectation, outcome)
    }

    /// Commit one selected newtype construction at its call site.
    fn commit_newtype_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        signature: NewtypeSignature,
        expectation: Option<Expectation>,
        outcome: CheckOutcome,
    ) -> CompilerResult<ValueCheck> {
        let NewtypeSignature {
            key,
            backing,
            arm,
            signature,
        } = signature;

        // deny a construction the backing's declared visibility rejects
        self.check_backing_access(origin, key.symbol)?;

        // commit conversions only after the backing has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        // commit the construction over the selected newtype backing and its arm
        let target = dir::ConstructTarget::Newtype { key, backing, arm };
        let arguments = signature.bind_arguments(Origin::Node(node, None), self)?;
        let resolution = dir::ConstructDecision::new(
            target,
            arguments,
            signature.return_type,
            signature.region_arguments.clone(),
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;

        let expected = expectation
            .and_then(Expectation::contextual_target)
            .unwrap_or(signature.return_type);

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
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        constructor: dir::ClassConstructor,
        signature: SignatureSelection,
        forms: &[dir::Form],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bind the constructor's chain, the class instance's arguments when inference stayed closed
        let arguments = if signature.generic_arguments.is_empty() {
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
        let target = self.class_construct_target(instance.symbol, constructor, arguments)?;

        // record an owned construction for the escape check at module end
        if forms.contains(&dir::Form::Owned) {
            self.owned_constructions.push((node, instance.symbol));
        }

        // wrap the produced instance in the destination forms, replacing its own heap form
        let mut produced = signature.return_type;

        for form in forms.iter().rev().copied() {
            produced = self.intern_type(dir::Type::Form(dir::FormType {
                form,
                value: produced,
            }))?;
        }

        // commit the construction over the selected class constructor
        let resolution = dir::ConstructDecision::new(
            target,
            signature.bind_arguments(Origin::Node(node, None), self)?,
            produced,
            signature.region_arguments.clone(),
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;
        self.commit_node_type(node, produced)?;

        Ok(produced)
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

        // delegate from the constructor of the derived class alone
        let Some(owner) = self.current_initializes() else {
            self.report_super_call_outside_constructor(module, node.local_id);
            self.infer_argument_types(site, argument_nodes)?;

            return self.poison_call(node, None);
        };
        if !self.class_has_base(owner)? {
            self.report_super_call_outside_constructor(module, node.local_id);
            self.infer_argument_types(site, argument_nodes)?;

            return self.poison_call(node, None);
        }
        self.flow.insert_assigned(AssignedPlace::Delegated);

        // type the super callee at its first visit
        let callee_site = self.visit_site(callee.into_global_any(module))?;
        self.infer_node_type(callee_site, PlaceUse::Read)?;

        // read the base receiver committed on the super callee
        let super_receiver = self.require_node_type(callee.into_global_any(module))?;

        // poison the call when the super type carries a reported error
        if matches!(self.ty(super_receiver)?, dir::Type::Error) {
            return self.poison_call(node, None);
        }

        // read the base class this super call initializes beneath the receiver's forms
        let super_ty = self.strip_form(origin, super_receiver)?;
        let (base_module, instance) = self.nominal_application(super_ty)?;
        // collect base constructors including forwarded defaults
        let mut active = SmallVec::new();
        let constructors = self.class_constructors(origin, super_ty, &instance, &mut active)?;

        // construct the base the super receiver names
        let expectation = Some(Expectation {
            target: super_ty,
            relation: Relation::Storable,
            cause: self.intern_cause(Cause::root(origin, CauseKind::Expression)),
            use_: ValueUse::Store,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        });

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
                    expectation,
                    Some(super_ty),
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
                base_module,
                &instance,
                constructor.constructor.clone(),
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

    /// Return whether one class declares a base class.
    pub(in crate::sema) fn class_has_base(
        &mut self,
        class: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        Ok(matches!(
            self.definition(class)?.as_deref(),
            Some(dir::Definition::Class(definition)) if definition.extends.is_some()
        ))
    }

    /// Commit one selected base constructor as the super initialization.
    fn commit_super_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        base_module: ModuleId,
        instance: &dir::GenericApplication,
        constructor: dir::ClassConstructor,
        signature: SignatureSelection,
    ) -> CompilerResult<ValueCheck> {
        // commit conversions only after the constructor has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        // bind the constructor's chain, the base instance's arguments when inference stayed closed
        let arguments = if signature.generic_arguments.is_empty() {
            let arguments: SmallVec<[_; 8]> =
                self.type_ids(base_module, instance.arguments)?.into();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };

        // initialize this through a super call, which produces void
        let produced = self.intern_type(dir::Type::Void)?;
        let target = self.class_construct_target(instance.symbol, constructor, arguments)?;
        let resolution = dir::ConstructDecision::new(
            target,
            signature.bind_arguments(Origin::Node(node, None), self)?,
            produced,
            signature.region_arguments.clone(),
        );
        self.commit_decision(node, dir::Decision::Construct(resolution))?;
        self.commit_node_type(node, produced)?;

        Ok(ValueCheck {
            source: produced,
            outcome: CheckOutcome::Holds,
            target: produced,
        })
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
        let symbol = match self.ty(target)?.symbol() {
            Some(symbol) => symbol,
            None => return Ok(None),
        };

        // reject the families aggregate literals never construct
        let hint = match self.definition(symbol)?.as_deref() {
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
    pub(in crate::sema) fn report_rejected_construct_target(
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
