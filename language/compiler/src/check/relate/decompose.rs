use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, Cause, CauseKind, CheckState, Constraint, GenericParameterId, Origin, Relation,
    TypeSubstitution, answer,
};

/// Applied generic argument that violates its declared parameter bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct GenericBoundRejection {
    /// The source node for the rejected argument.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The rejected argument type.
    pub(in crate::check) argument: dir::GlobalTypeId,
    /// The required bound after earlier arguments were substituted.
    pub(in crate::check) bound: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Decompose two same-constructor types into fixed slot pairs.
    pub(in crate::check) fn decompose_type_pair(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        let pair_lists = match (self.ty(source)?, self.ty(target)?) {
            // nominal applications decompose by declaration
            (dir::Type::Instance(source_type), dir::Type::Instance(target_type))
                if source_type.symbol == target_type.symbol =>
            {
                let source = self.type_ids(source.module_id, source_type.arguments)?;
                let target = self.type_ids(target.module_id, target_type.arguments)?;

                (SmallVec::from_slice(source), SmallVec::from_slice(target))
            }

            // member projections decompose by key, owner, arguments, and qualifier
            (dir::Type::Member(source_type), dir::Type::Member(target_type))
                if let source_type = self.type_member(source.module_id, source_type)?
                    && let target_type = self.type_member(target.module_id, target_type)?
                    && source_type.key == target_type.key
                    && source_type.qualifier.is_some() == target_type.qualifier.is_some() =>
            {
                let mut source_slots = SmallVec::from_slice(&[source_type.owner]);
                source_slots
                    .extend_from_slice(self.type_ids(source.module_id, source_type.arguments)?);
                source_slots.extend(source_type.qualifier);
                let mut target_slots = SmallVec::from_slice(&[target_type.owner]);
                target_slots
                    .extend_from_slice(self.type_ids(target.module_id, target_type.arguments)?);
                target_slots.extend(target_type.qualifier);

                (source_slots, target_slots)
            }

            // enum members decompose by member and owner
            (dir::Type::EnumMember(source_type), dir::Type::EnumMember(target_type))
                if source_type.member == target_type.member =>
            {
                (
                    SmallVec::from_slice(&[source_type.owner]),
                    SmallVec::from_slice(&[target_type.owner]),
                )
            }

            // memory forms decompose by constructor: borrow lifetimes,
            //  accesses, and places are type-valued fixed slots
            (dir::Type::Form(source_type), dir::Type::Form(target_type)) => {
                match (source_type.form, target_type.form) {
                    (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) => {
                        let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                        let target_borrow = self.type_borrow(target.module_id, target_borrow)?;

                        (
                            SmallVec::from_slice(&[
                                source_borrow.lifetime,
                                source_borrow.access,
                                source_type.value,
                            ]),
                            SmallVec::from_slice(&[
                                target_borrow.lifetime,
                                target_borrow.access,
                                target_type.value,
                            ]),
                        )
                    }
                    (
                        dir::Form::Placed { place },
                        dir::Form::Placed {
                            place: target_place,
                        },
                    ) => (
                        SmallVec::from_slice(&[place, source_type.value]),
                        SmallVec::from_slice(&[target_place, target_type.value]),
                    ),
                    (source_form, target_form) if source_form == target_form => (
                        SmallVec::from_slice(&[source_type.value]),
                        SmallVec::from_slice(&[target_type.value]),
                    ),
                    _ => return Ok(None),
                }
            }

            // dynamic representations decompose over their constraints
            (dir::Type::Dynamic(source_type), dir::Type::Dynamic(target_type)) => (
                SmallVec::from_slice(&[source_type.constraint]),
                SmallVec::from_slice(&[target_type.constraint]),
            ),

            // callables decompose over their signatures and environments
            (dir::Type::Function(source_type), dir::Type::Function(target_type)) => (
                SmallVec::from_slice(&[source_type.signature, source_type.environment]),
                SmallVec::from_slice(&[target_type.signature, target_type.environment]),
            ),
            (dir::Type::FunctionPointer(source_type), dir::Type::FunctionPointer(target_type)) => (
                SmallVec::from_slice(&[source_type.signature]),
                SmallVec::from_slice(&[target_type.signature]),
            ),

            // type operations decompose after non-type payloads agree
            (dir::Type::Operation(source_type), dir::Type::Operation(target_type)) => {
                let source_type = self.type_operation(source.module_id, source_type)?;
                let target_type = self.type_operation(target.module_id, target_type)?;

                return self.decompose_operation_pair(
                    source.module_id,
                    &source_type,
                    target.module_id,
                    &target_type,
                );
            }

            // value containers decompose over their contained types
            (dir::Type::Array(source_type), dir::Type::Array(target_type)) => (
                SmallVec::from_slice(&[source_type.element]),
                SmallVec::from_slice(&[target_type.element]),
            ),
            (dir::Type::Slice(source_type), dir::Type::Slice(target_type)) => (
                SmallVec::from_slice(&[source_type.element]),
                SmallVec::from_slice(&[target_type.element]),
            ),
            (dir::Type::FixedArray(source_type), dir::Type::FixedArray(target_type)) => (
                SmallVec::from_slice(&[source_type.element, source_type.count]),
                SmallVec::from_slice(&[target_type.element, target_type.count]),
            ),

            // tuples decompose element-wise when their element shapes agree
            (dir::Type::Tuple(source_type), dir::Type::Tuple(target_type))
                if source_type.form == target_type.form
                    && source_type.elements.len() == target_type.elements.len() =>
            {
                let source_elements =
                    self.tuple_elements(source.module_id, source_type.elements)?;
                let target_elements =
                    self.tuple_elements(target.module_id, target_type.elements)?;
                let mut source_slots = SmallVec::new();
                let mut target_slots = SmallVec::new();
                for (source, target) in source_elements.iter().zip(target_elements) {
                    if source.label != target.label
                        || source.is_optional != target.is_optional
                        || source.is_readonly != target.is_readonly
                        || source.is_rest != target.is_rest
                    {
                        return Ok(None);
                    }
                    source_slots.push(source.ty);
                    target_slots.push(target.ty);
                }

                (source_slots, target_slots)
            }

            // signatures decompose inputs and outputs when their shapes agree
            (
                dir::Type::FunctionSignature(source_type),
                dir::Type::FunctionSignature(target_type),
            ) if let source_type = self.type_signature(source.module_id, source_type)?
                && let target_type = self.type_signature(target.module_id, target_type)?
                && source_type.this_parameter.is_some() == target_type.this_parameter.is_some()
                && source_type.return_type.is_some() == target_type.return_type.is_some() =>
            {
                let source_parameters =
                    self.signature_parameters(source.module_id, source_type.parameters)?;
                let target_parameters =
                    self.signature_parameters(target.module_id, target_type.parameters)?;
                if source_parameters.len() != target_parameters.len() {
                    return Ok(None);
                }
                let mut source_slots = SmallVec::new();
                let mut target_slots = SmallVec::new();
                source_slots.extend(source_type.this_parameter);
                target_slots.extend(target_type.this_parameter);
                for (source, target) in source_parameters.iter().zip(target_parameters) {
                    if source.is_optional != target.is_optional || source.is_rest != target.is_rest
                    {
                        return Ok(None);
                    }
                    source_slots.push(source.ty);
                    target_slots.push(target.ty);
                }
                source_slots.extend(source_type.return_type);
                target_slots.extend(target_type.return_type);

                (source_slots, target_slots)
            }

            // set types decompose element-wise in written order
            (dir::Type::Union(source_type), dir::Type::Union(target_type))
                if source_type.elements.len() == target_type.elements.len() =>
            {
                let source = self.type_ids(source.module_id, source_type.elements)?;
                let target = self.type_ids(target.module_id, target_type.elements)?;

                (SmallVec::from_slice(source), SmallVec::from_slice(target))
            }
            (dir::Type::Intersection(source_type), dir::Type::Intersection(target_type))
                if source_type.elements.len() == target_type.elements.len() =>
            {
                let source = self.type_ids(source.module_id, source_type.elements)?;
                let target = self.type_ids(target.module_id, target_type.elements)?;

                (SmallVec::from_slice(source), SmallVec::from_slice(target))
            }

            _ => return Ok(None),
        };

        Ok(type_pairs(pair_lists.0, pair_lists.1))
    }

    /// Decompose two fixed-arity type operations under one shared constructor.
    fn decompose_operation_pair(
        &self,
        source_module: ModuleId,
        source: &dir::TypeOperation,
        target_module: ModuleId,
        target: &dir::TypeOperation,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        let pair_lists =
            match (source, target) {
                // string mappings decompose over their mapped target
                (
                    dir::TypeOperation::StringMapping {
                        mapping: source_mapping,
                        target: source_target,
                    },
                    dir::TypeOperation::StringMapping {
                        mapping: target_mapping,
                        target: target_target,
                    },
                ) if source_mapping == target_mapping => (
                    SmallVec::from_slice(&[*source_target]),
                    SmallVec::from_slice(&[*target_target]),
                ),

                // conditionals decompose operands and branches
                (
                    dir::TypeOperation::Conditional(source),
                    dir::TypeOperation::Conditional(target),
                ) if source.is_distributive == target.is_distributive => (
                    SmallVec::from_slice(&[
                        source.left,
                        source.right,
                        source.then_type,
                        source.else_type,
                    ]),
                    SmallVec::from_slice(&[
                        target.left,
                        target.right,
                        target.then_type,
                        target.else_type,
                    ]),
                ),

                // narrows decompose source and target under one polarity
                (dir::TypeOperation::Narrow(source), dir::TypeOperation::Narrow(target))
                    if source.is_positive == target.is_positive =>
                {
                    (
                        SmallVec::from_slice(&[source.source, source.target]),
                        SmallVec::from_slice(&[target.source, target.target]),
                    )
                }

                // mapped types decompose constraint, key remap, and value under one binder
                (dir::TypeOperation::Mapped(source), dir::TypeOperation::Mapped(target))
                    if source.parameter.name == target.parameter.name
                        && source.parameter.parameter == target.parameter.parameter
                        && source.modifiers == target.modifiers
                        && source.parameter.key_remap.is_some()
                            == target.parameter.key_remap.is_some() =>
                {
                    let mut source_slots = SmallVec::from_slice(&[source.parameter.constraint]);
                    source_slots.extend(source.parameter.key_remap);
                    source_slots.push(source.value);
                    let mut target_slots = SmallVec::from_slice(&[target.parameter.constraint]);
                    target_slots.extend(target.parameter.key_remap);
                    target_slots.push(target.value);

                    (source_slots, target_slots)
                }

                // indexed accesses decompose receiver and index
                (dir::TypeOperation::Index(source), dir::TypeOperation::Index(target)) => (
                    SmallVec::from_slice(&[source.left, source.index]),
                    SmallVec::from_slice(&[target.left, target.index]),
                ),

                // type queries compare by referenced source path
                (dir::TypeOperation::TypeOf(source), dir::TypeOperation::TypeOf(target))
                    if source.value == target.value =>
                {
                    (SmallVec::new(), SmallVec::new())
                }

                // template literals decompose spans under equal strings
                (
                    dir::TypeOperation::TemplateLiteral(source),
                    dir::TypeOperation::TemplateLiteral(target),
                ) if self.template_strings(source_module, source.strings)?
                    == self.template_strings(target_module, target.strings)? =>
                {
                    (
                        SmallVec::from_slice(self.type_ids(source_module, source.spans)?),
                        SmallVec::from_slice(self.type_ids(target_module, target.spans)?),
                    )
                }

                // infer binders decompose their optional constraint under one name
                (dir::TypeOperation::Infer(source), dir::TypeOperation::Infer(target))
                    if source.name == target.name
                        && source.constraint.is_some() == target.constraint.is_some() =>
                {
                    (
                        SmallVec::from_iter(source.constraint),
                        SmallVec::from_iter(target.constraint),
                    )
                }

                // unary operations decompose their targets
                (dir::TypeOperation::KeyOf(source), dir::TypeOperation::KeyOf(target))
                | (dir::TypeOperation::NoInfer(source), dir::TypeOperation::NoInfer(target))
                | (dir::TypeOperation::Awaited(source), dir::TypeOperation::Awaited(target)) => (
                    SmallVec::from_slice(&[source.target]),
                    SmallVec::from_slice(&[target.target]),
                ),

                // try projections decompose their projected value
                (
                    dir::TypeOperation::TryOutput { value: source },
                    dir::TypeOperation::TryOutput { value: target },
                )
                | (
                    dir::TypeOperation::TryResidual { value: source },
                    dir::TypeOperation::TryResidual { value: target },
                ) => (
                    SmallVec::from_slice(&[*source]),
                    SmallVec::from_slice(&[*target]),
                ),

                // static binary operations decompose operands under one operator
                (
                    dir::TypeOperation::StaticBinary(source),
                    dir::TypeOperation::StaticBinary(target),
                ) if source.operator == target.operator => (
                    SmallVec::from_slice(&[source.left, source.right]),
                    SmallVec::from_slice(&[target.left, target.right]),
                ),

                // static unary operations decompose targets under one operator
                (
                    dir::TypeOperation::StaticUnary(source),
                    dir::TypeOperation::StaticUnary(target),
                ) if source.operator == target.operator => (
                    SmallVec::from_slice(&[source.target]),
                    SmallVec::from_slice(&[target.target]),
                ),

                _ => return Ok(None),
            };

        Ok(type_pairs(pair_lists.0, pair_lists.1))
    }

    /// Check applied generic arguments against declared parameter bounds.
    pub(in crate::check) fn check_generic_arguments(
        &mut self,
        origin: Origin,
        parameters: &[dir::GlobalGenericParameterId],
        arguments: &[dir::GlobalTypeId],
        sources: &[dir::GlobalNodeIdAny],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<Option<GenericBoundRejection>>> {
        for ((parameter, argument), argument_source) in parameters
            .iter()
            .copied()
            .zip(arguments.iter().copied())
            .zip(sources.iter().copied())
        {
            let Some(bound) =
                self.substituted_parameter_bound(origin.module(), parameter, substitution)?
            else {
                continue;
            };

            let origin = self.origin_at(origin, argument_source)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            match self.constrain_type(cause, Relation::Satisfies, argument, bound)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => {
                    return Ok(Answer::Ready(Some(GenericBoundRejection {
                        source: argument_source,
                        argument,
                        bound,
                    })));
                }
                Answer::Pending(blockers) if self.solver.is_probing() => {
                    return Ok(Answer::Pending(blockers));
                }
                // park undecidable bounds for fulfillment outside probes
                Answer::Pending(_) => {
                    let cause =
                        self.intern_cause(Cause::root(origin, CauseKind::Bound { parameter }));
                    self.push_constraint(Constraint::r#type(
                        Relation::Satisfies,
                        argument,
                        bound,
                        cause,
                    ));
                }
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Match one generic type pattern and return its direct substitution.
    pub(in crate::check) fn match_generic_pattern(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        self.match_generic_pairs(origin, parameters, &[(pattern, actual)])
    }

    /// Match one parameter list over positional pattern and actual pairs.
    pub(in crate::check) fn match_generic_pairs(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        let mut substitution = TypeSubstitution::default();
        for (pattern, actual) in pairs.iter().copied() {
            if !answer!(self.match_generic_type(
                origin,
                parameters,
                &mut substitution,
                pattern,
                actual,
            )?) {
                return Ok(Answer::Ready(None));
            }
        }

        // matched substitutions must still satisfy the declared bounds
        let source = self.origin_source_node(origin)?;
        if !answer!(self.check_generic_substitution_bounds(
            origin,
            source.into_global(origin.module()),
            &substitution,
        )?) {
            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Return the type substitution for one generic instance.
    pub(in crate::check) fn instance_substitution(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<TypeSubstitution> {
        let Some(template) = self.symbol_template(instance.symbol)? else {
            return Ok(TypeSubstitution::default());
        };

        let parameters = self.generic_template_parameters(template);
        let arguments = self
            .type_ids(module, instance.arguments)?
            .iter()
            .copied()
            .take(parameters.len())
            .collect();

        Ok(TypeSubstitution {
            parameters,
            arguments,
            receiver: None,
        })
    }

    /// Return one instance substitution with omitted arguments defaulted.
    pub(in crate::check) fn instance_substitution_with_defaults(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericInstance,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<TypeSubstitution> {
        let mut substitution = self.instance_substitution(module, instance)?;
        substitution.receiver = receiver;

        // resolve receiver-relative arguments at this application
        if let Some(receiver) = receiver {
            let receiver_only = TypeSubstitution::default().with_receiver(receiver);
            for argument in substitution.arguments.iter_mut() {
                *argument = self.substitute_type(module, *argument, &receiver_only)?;
            }
        }

        // evaluate omitted defaults against the application built so far
        let parameters = substitution.parameters.clone();
        for parameter in parameters
            .iter()
            .skip(substitution.arguments.len())
            .copied()
        {
            let Some(binding) = self.generic_parameter(parameter).copied() else {
                break;
            };
            let Some(default) = binding.default else {
                break;
            };
            let default = self.substitute_type(module, default, &substitution)?;
            substitution.arguments.push(default);
        }

        Ok(substitution)
    }

    /// Match one generic type pattern without opening inference variables.
    fn match_generic_type(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let pattern = answer!(self.reduce_type(origin, pattern)?);
        // open actuals reduce heads only so bounds can flow into their holes
        let actual = self.settled_root(actual)?;
        let actual = if self.root_variable(actual)?.is_some() {
            actual
        } else if self.type_variables(actual)?.is_empty() {
            answer!(self.reduce_type(origin, actual)?)
        } else {
            answer!(self.reduce_type_head(origin, actual)?)
        };

        let pattern_type = self.ty(pattern)?;
        let actual_type = self.ty(actual)?;

        // lifetime slots collect components and verify outlives on MIR,
        //  so they never gate matching: elided implementation lifetimes
        //  serve any spread of required ones
        if self.is_lifetime_slot(&pattern_type)? && self.is_lifetime_slot(&actual_type)? {
            return Ok(Answer::Ready(true));
        }

        // bind template parameters directly
        if let dir::Type::Parameter(parameter) = pattern_type
            && parameters.contains(&parameter)
        {
            return self.bind_generic_argument(origin, substitution, parameter, actual);
        }

        // decompose identical parameterized types to record bindings
        if pattern == actual && !self.type_flags(pattern)?.has_parameter() {
            return Ok(Answer::Ready(true));
        }

        // decompose fixed slots beneath one shared constructor
        let pairs = self.decompose_type_pair(pattern, actual)?;
        if let Some(pairs) = pairs {
            return self.match_generic_arguments(origin, parameters, substitution, &pairs);
        }

        Ok(Answer::Ready(pattern_type == actual_type))
    }

    /// Return whether one matched slot is lifetime-shaped.
    pub(in crate::check) fn is_lifetime_slot(&self, ty: &dir::Type) -> CompilerResult<bool> {
        match ty {
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)) => Ok(true),
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                Ok(self.generic_parameter(*parameter).is_some_and(|binding| {
                    binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                }))
            }
            _ => Ok(false),
        }
    }

    /// Bind one generic parameter argument in a direct substitution.
    fn bind_generic_argument(
        &mut self,
        origin: Origin,
        substitution: &mut TypeSubstitution,
        parameter: GenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let position = substitution
            .parameters
            .iter()
            .position(|candidate| *candidate == parameter);
        let Some(position) = position else {
            substitution.parameters.push(parameter);
            substitution.arguments.push(argument);

            return Ok(Answer::Ready(true));
        };

        let bound = substitution.arguments[position];

        self.decide_relation(origin, Relation::Equal, bound, argument)
    }

    /// Check every generic argument in one completed substitution.
    fn check_generic_substitution_bounds(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<bool>> {
        for (parameter, argument) in substitution
            .parameters
            .iter()
            .copied()
            .zip(substitution.arguments.iter().copied())
        {
            let Some(bound) =
                self.substituted_parameter_bound(origin.module(), parameter, substitution)?
            else {
                continue;
            };
            let anchored = self.origin_at(origin, source)?;
            let cause = self.intern_cause(Cause::root(anchored, CauseKind::Bound { parameter }));
            if !answer!(self.constrain_type(cause, Relation::Satisfies, argument, bound)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Return one parameter's declared bound under a substitution.
    fn substituted_parameter_bound(
        &mut self,
        module: ModuleId,
        parameter: dir::GlobalGenericParameterId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(constraint) = self
            .generic_parameter(parameter)
            .and_then(|binding| binding.constraint)
        else {
            return Ok(None);
        };

        Ok(Some(self.substitute_type(
            module,
            constraint,
            substitution,
        )?))
    }

    /// Match fixed positional type pairs.
    fn match_generic_arguments(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pairs.iter().copied() {
            decision = decision.and(self.match_generic_type(
                origin,
                parameters,
                substitution,
                pattern,
                actual,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}

/// Zip two fixed slot lists into relation pairs.
fn type_pairs(
    source: SmallVec<[dir::GlobalTypeId; 4]>,
    target: SmallVec<[dir::GlobalTypeId; 4]>,
) -> Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>> {
    if source.len() != target.len() {
        return None;
    }

    Some(source.into_iter().zip(target).collect())
}
