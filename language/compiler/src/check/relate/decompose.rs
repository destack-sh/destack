use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, GenericParameterId, Origin, Relation, TypeSubstitution, answer,
};

impl CheckState<'_> {
    /// Decompose two same-constructor types into fixed slot pairs.
    pub(in crate::check) fn decompose_type_pair(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        let pair_lists = match (self.ty(source)?, self.ty(target)?) {
            // nominal applications decompose by declaration
            (dir::Type::Application(source_type), dir::Type::Application(target_type))
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

            // variants decompose by member and owner
            (dir::Type::Variant(source_type), dir::Type::Variant(target_type))
                if source_type.variant == target_type.variant =>
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
        if !answer!(self.extend_generic_substitution(
            origin,
            parameters,
            &mut substitution,
            pairs,
        )?) {
            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Extend one substitution by structurally matching positional type pairs.
    pub(in crate::check) fn extend_generic_substitution(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        for (pattern, actual) in pairs.iter().copied() {
            if !answer!(self.match_generic_type(
                origin,
                parameters,
                substitution,
                pattern,
                actual,
            )?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
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

        // unions and intersections match as unordered type sets
        let set = match (pattern_type, actual_type) {
            (dir::Type::Union(pattern), dir::Type::Union(actual)) => {
                Some((pattern.elements, actual.elements))
            }
            (dir::Type::Intersection(pattern), dir::Type::Intersection(actual)) => {
                Some((pattern.elements, actual.elements))
            }
            _ => None,
        };
        if let Some((pattern_elements, actual_elements)) = set {
            let pattern_elements =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(pattern.module_id, pattern_elements)?);
            let actual_elements =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(actual.module_id, actual_elements)?);

            return self.match_generic_type_sets(
                origin,
                parameters,
                substitution,
                pattern_elements,
                actual_elements,
            );
        }

        // decompose fixed slots beneath one shared constructor
        let pairs = self.decompose_type_pair(pattern, actual)?;
        if let Some(pairs) = pairs {
            return self.match_generic_arguments(origin, parameters, substitution, &pairs);
        }

        Ok(Answer::Ready(pattern_type == actual_type))
    }

    /// Match two unordered type sets wherever each pattern has one viable target.
    fn match_generic_type_sets(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        mut patterns: SmallVec<[dir::GlobalTypeId; 4]>,
        mut actuals: SmallVec<[dir::GlobalTypeId; 4]>,
    ) -> CompilerResult<Answer<bool>> {
        if patterns.len() != actuals.len() {
            return Ok(Answer::Ready(false));
        }

        // commit only matches whose target is unambiguous under current bindings
        while !patterns.is_empty() {
            let mut selected = None;
            let mut blockers = SmallVec::<[Dependency; 2]>::new();
            for (pattern_index, pattern) in patterns.iter().copied().enumerate() {
                let mut candidate = None;
                let mut is_ambiguous = false;
                for (actual_index, actual) in actuals.iter().copied().enumerate() {
                    let mut matched = substitution.clone();
                    match self.match_generic_type(
                        origin,
                        parameters,
                        &mut matched,
                        pattern,
                        actual,
                    )? {
                        Answer::Ready(true) if candidate.is_some() => is_ambiguous = true,
                        Answer::Ready(true) => candidate = Some((actual_index, matched)),
                        Answer::Ready(false) => {}
                        Answer::Pending(dependencies) => blockers.extend(dependencies),
                    }
                }
                if !is_ambiguous && let Some((actual_index, matched)) = candidate {
                    selected = Some((pattern_index, actual_index, matched));

                    break;
                }
            }

            // consume one unambiguous pair or wait for unresolved evidence
            if let Some((pattern_index, actual_index, matched)) = selected {
                *substitution = matched;
                patterns.remove(pattern_index);
                actuals.remove(actual_index);
            } else if !blockers.is_empty() {
                return Ok(Answer::Pending(blockers));
            } else {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
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
        let Some(bound) = substitution.argument(parameter) else {
            substitution.bind(parameter, argument)?;

            return Ok(Answer::Ready(true));
        };

        self.decide_relation(origin, Relation::Equal, bound, argument)
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
