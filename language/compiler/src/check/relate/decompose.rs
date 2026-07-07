use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, GenericParameterId, Origin, Relation, TypeSubstitution, answer,
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
    ///
    /// Generic parameter packs require parameter-list matching, not fixed slot decomposition.
    pub(in crate::check) fn decompose_type_pair(
        &self,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        let pair_lists = match (self.ty(left)?, self.ty(right)?) {
            // nominal applications decompose by declaration
            (dir::Type::Instance(left_type), dir::Type::Instance(right_type))
                if left_type.symbol == right_type.symbol =>
            {
                let left = self.type_ids(left.module_id, left_type.arguments)?;
                let right = self.type_ids(right.module_id, right_type.arguments)?;

                (SmallVec::from_slice(left), SmallVec::from_slice(right))
            }

            // member projections decompose by key, owner, arguments, and qualifier
            (dir::Type::Member(left_type), dir::Type::Member(right_type))
                if left_type.key == right_type.key
                    && left_type.qualifier.is_some() == right_type.qualifier.is_some() =>
            {
                let mut left_slots = SmallVec::from_slice(&[left_type.owner]);
                left_slots.extend_from_slice(self.type_ids(left.module_id, left_type.arguments)?);
                left_slots.extend(left_type.qualifier);
                let mut right_slots = SmallVec::from_slice(&[right_type.owner]);
                right_slots
                    .extend_from_slice(self.type_ids(right.module_id, right_type.arguments)?);
                right_slots.extend(right_type.qualifier);

                (left_slots, right_slots)
            }

            // enum members decompose by member and owner
            (dir::Type::EnumMember(left_type), dir::Type::EnumMember(right_type))
                if left_type.member == right_type.member =>
            {
                (
                    SmallVec::from_slice(&[left_type.owner]),
                    SmallVec::from_slice(&[right_type.owner]),
                )
            }

            // memory forms decompose by constructor: borrow lifetimes,
            // accesses, and places are type-valued fixed slots
            (dir::Type::Form(left_type), dir::Type::Form(right_type)) => {
                match (left_type.form, right_type.form) {
                    (
                        dir::Form::Borrowed { lifetime, access },
                        dir::Form::Borrowed {
                            lifetime: right_lifetime,
                            access: right_access,
                        },
                    ) => (
                        SmallVec::from_slice(&[lifetime, access, left_type.value]),
                        SmallVec::from_slice(&[right_lifetime, right_access, right_type.value]),
                    ),
                    (dir::Form::Placed { place }, dir::Form::Placed { place: right_place }) => (
                        SmallVec::from_slice(&[place, left_type.value]),
                        SmallVec::from_slice(&[right_place, right_type.value]),
                    ),
                    (left_form, right_form) if left_form == right_form => (
                        SmallVec::from_slice(&[left_type.value]),
                        SmallVec::from_slice(&[right_type.value]),
                    ),
                    _ => return Ok(None),
                }
            }

            // dynamic representations decompose over their constraints
            (dir::Type::Dynamic(left_type), dir::Type::Dynamic(right_type)) => (
                SmallVec::from_slice(&[left_type.constraint]),
                SmallVec::from_slice(&[right_type.constraint]),
            ),

            // callables decompose over their signatures and environments
            (dir::Type::Function(left_type), dir::Type::Function(right_type)) => (
                SmallVec::from_slice(&[left_type.signature, left_type.environment]),
                SmallVec::from_slice(&[right_type.signature, right_type.environment]),
            ),
            (dir::Type::FunctionPointer(left_type), dir::Type::FunctionPointer(right_type)) => (
                SmallVec::from_slice(&[left_type.signature]),
                SmallVec::from_slice(&[right_type.signature]),
            ),

            // type operations decompose after non-type payloads agree
            (dir::Type::Operation(left_type), dir::Type::Operation(right_type)) => {
                return self.decompose_operation_pair(
                    left.module_id,
                    &left_type,
                    right.module_id,
                    &right_type,
                );
            }

            // value containers decompose over their contained types
            (dir::Type::Array(left_type), dir::Type::Array(right_type)) => (
                SmallVec::from_slice(&[left_type.element]),
                SmallVec::from_slice(&[right_type.element]),
            ),
            (dir::Type::Slice(left_type), dir::Type::Slice(right_type)) => (
                SmallVec::from_slice(&[left_type.element]),
                SmallVec::from_slice(&[right_type.element]),
            ),
            (dir::Type::FixedArray(left_type), dir::Type::FixedArray(right_type)) => (
                SmallVec::from_slice(&[left_type.element, left_type.count]),
                SmallVec::from_slice(&[right_type.element, right_type.count]),
            ),

            // tuples decompose element-wise when their element shapes agree
            (dir::Type::Tuple(left_type), dir::Type::Tuple(right_type))
                if left_type.form == right_type.form
                    && left_type.elements.len() == right_type.elements.len() =>
            {
                let left_elements = self.tuple_elements(left.module_id, left_type.elements)?;
                let right_elements = self.tuple_elements(right.module_id, right_type.elements)?;
                let mut left_slots = SmallVec::new();
                let mut right_slots = SmallVec::new();
                for (left, right) in left_elements.iter().zip(right_elements) {
                    if left.label != right.label
                        || left.is_optional != right.is_optional
                        || left.is_readonly != right.is_readonly
                        || left.is_rest != right.is_rest
                    {
                        return Ok(None);
                    }
                    left_slots.push(left.ty);
                    right_slots.push(right.ty);
                }

                (left_slots, right_slots)
            }

            // signatures decompose inputs and outputs when their shapes agree
            (dir::Type::FunctionSignature(left_type), dir::Type::FunctionSignature(right_type))
                if left_type.asynchrony == right_type.asynchrony
                    && left_type.is_generator == right_type.is_generator
                    && left_type.this_parameter.is_some()
                        == right_type.this_parameter.is_some()
                    && left_type.return_type.is_some() == right_type.return_type.is_some() =>
            {
                let left_parameters =
                    self.signature_parameters(left.module_id, left_type.parameters)?;
                let right_parameters =
                    self.signature_parameters(right.module_id, right_type.parameters)?;
                if left_parameters.len() != right_parameters.len() {
                    return Ok(None);
                }
                let mut left_slots = SmallVec::new();
                let mut right_slots = SmallVec::new();
                left_slots.extend(left_type.this_parameter);
                right_slots.extend(right_type.this_parameter);
                for (left, right) in left_parameters.iter().zip(right_parameters) {
                    if left.is_optional != right.is_optional || left.is_rest != right.is_rest {
                        return Ok(None);
                    }
                    left_slots.push(left.ty);
                    right_slots.push(right.ty);
                }
                left_slots.extend(left_type.return_type);
                right_slots.extend(right_type.return_type);

                (left_slots, right_slots)
            }

            // set types decompose element-wise in written order
            (dir::Type::Union(left_type), dir::Type::Union(right_type))
                if left_type.elements.len() == right_type.elements.len() =>
            {
                let left = self.type_ids(left.module_id, left_type.elements)?;
                let right = self.type_ids(right.module_id, right_type.elements)?;

                (SmallVec::from_slice(left), SmallVec::from_slice(right))
            }
            (dir::Type::Intersection(left_type), dir::Type::Intersection(right_type))
                if left_type.elements.len() == right_type.elements.len() =>
            {
                let left = self.type_ids(left.module_id, left_type.elements)?;
                let right = self.type_ids(right.module_id, right_type.elements)?;

                (SmallVec::from_slice(left), SmallVec::from_slice(right))
            }

            _ => return Ok(None),
        };

        Ok(type_pairs(pair_lists.0, pair_lists.1))
    }

    /// Decompose two fixed-arity type operations under one shared constructor.
    ///
    /// Operations decompose exactly when their constructor and non-type
    /// payload agree. The module arguments name where each list payload lives.
    fn decompose_operation_pair(
        &self,
        left_module: destack_source::ModuleId,
        left: &dir::TypeOperation,
        right_module: destack_source::ModuleId,
        right: &dir::TypeOperation,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        let pair_lists = match (left, right) {
            // string mappings decompose over their mapped target
            (
                dir::TypeOperation::StringMapping {
                    mapping: left_mapping,
                    target: left_target,
                },
                dir::TypeOperation::StringMapping {
                    mapping: right_mapping,
                    target: right_target,
                },
            ) if left_mapping == right_mapping => (
                SmallVec::from_slice(&[*left_target]),
                SmallVec::from_slice(&[*right_target]),
            ),

            // conditionals decompose operands and branches
            (dir::TypeOperation::Conditional(left), dir::TypeOperation::Conditional(right))
                if left.is_distributive == right.is_distributive =>
            {
                (
                    SmallVec::from_slice(&[left.left, left.right, left.then_type, left.else_type]),
                    SmallVec::from_slice(&[
                        right.left,
                        right.right,
                        right.then_type,
                        right.else_type,
                    ]),
                )
            }

            // narrows decompose source and target under one polarity
            (dir::TypeOperation::Narrow(left), dir::TypeOperation::Narrow(right))
                if left.is_positive == right.is_positive =>
            {
                (
                    SmallVec::from_slice(&[left.source, left.target]),
                    SmallVec::from_slice(&[right.source, right.target]),
                )
            }

            // mapped types decompose constraint, key remap, and value under one binder
            (dir::TypeOperation::Mapped(left), dir::TypeOperation::Mapped(right))
                if left.parameter.name == right.parameter.name
                    && left.parameter.parameter == right.parameter.parameter
                    && left.modifiers == right.modifiers
                    && left.parameter.key_remap.is_some()
                        == right.parameter.key_remap.is_some() =>
            {
                let mut left_slots = SmallVec::from_slice(&[left.parameter.constraint]);
                left_slots.extend(left.parameter.key_remap);
                left_slots.push(left.value);
                let mut right_slots = SmallVec::from_slice(&[right.parameter.constraint]);
                right_slots.extend(right.parameter.key_remap);
                right_slots.push(right.value);

                (left_slots, right_slots)
            }

            // indexed accesses decompose receiver and index
            (dir::TypeOperation::Index(left), dir::TypeOperation::Index(right)) => (
                SmallVec::from_slice(&[left.left, left.index]),
                SmallVec::from_slice(&[right.left, right.index]),
            ),

            // type queries compare by referenced source path
            (dir::TypeOperation::TypeOf(left), dir::TypeOperation::TypeOf(right))
                if left.value == right.value =>
            {
                (SmallVec::new(), SmallVec::new())
            }

            // template literals decompose spans under equal strings
            (
                dir::TypeOperation::TemplateLiteral(left),
                dir::TypeOperation::TemplateLiteral(right),
            ) if self.template_strings(left_module, left.strings)?
                == self.template_strings(right_module, right.strings)? =>
            {
                (
                    SmallVec::from_slice(self.type_ids(left_module, left.spans)?),
                    SmallVec::from_slice(self.type_ids(right_module, right.spans)?),
                )
            }

            // infer binders decompose their optional constraint under one name
            (dir::TypeOperation::Infer(left), dir::TypeOperation::Infer(right))
                if left.name == right.name
                    && left.constraint.is_some() == right.constraint.is_some() =>
            {
                (
                    SmallVec::from_iter(left.constraint),
                    SmallVec::from_iter(right.constraint),
                )
            }

            // unary operations decompose their targets
            (dir::TypeOperation::KeyOf(left), dir::TypeOperation::KeyOf(right))
            | (dir::TypeOperation::NoInfer(left), dir::TypeOperation::NoInfer(right))
            | (dir::TypeOperation::Awaited(left), dir::TypeOperation::Awaited(right)) => (
                SmallVec::from_slice(&[left.target]),
                SmallVec::from_slice(&[right.target]),
            ),

            // try projections decompose their projected value
            (
                dir::TypeOperation::TryOutput { value: left },
                dir::TypeOperation::TryOutput { value: right },
            )
            | (
                dir::TypeOperation::TryResidual { value: left },
                dir::TypeOperation::TryResidual { value: right },
            ) => (
                SmallVec::from_slice(&[*left]),
                SmallVec::from_slice(&[*right]),
            ),

            // static binary operations decompose operands under one operator
            (dir::TypeOperation::StaticBinary(left), dir::TypeOperation::StaticBinary(right))
                if left.operator == right.operator =>
            {
                (
                    SmallVec::from_slice(&[left.left, left.right]),
                    SmallVec::from_slice(&[right.left, right.right]),
                )
            }

            // static unary operations decompose targets under one operator
            (dir::TypeOperation::StaticUnary(left), dir::TypeOperation::StaticUnary(right))
                if left.operator == right.operator =>
            {
                (
                    SmallVec::from_slice(&[left.target]),
                    SmallVec::from_slice(&[right.target]),
                )
            }

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
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let bound = self.substitute_type(origin.module(), constraint, substitution)?;

            let origin = self.origin_at(origin, argument_source);
            if !answer!(self.constrain_type(origin, Relation::Satisfies, argument, bound)?) {
                return Ok(Answer::Ready(Some(GenericBoundRejection {
                    source: argument_source,
                    argument,
                    bound,
                })));
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
        let mut substitution = TypeSubstitution::default();
        if !answer!(self.match_generic_type(
            origin,
            parameters,
            &mut substitution,
            pattern,
            actual
        )?) {
            return Ok(Answer::Ready(None));
        }

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

    /// Match one parameter list over positional pattern and actual pairs.
    ///
    /// Parameters the pairs leave free stay rigid, and bound arguments must
    /// satisfy their declared constraints.
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
    /// `module` is the owner of the instance's argument list.
    pub(in crate::check) fn instance_substitution(
        &self,
        module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<TypeSubstitution> {
        let Some(template) = self.symbol_template(instance.symbol) else {
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
        let actual = answer!(self.reduce_type(origin, actual)?);

        let pattern_type = self.ty(pattern)?;
        let actual_type = self.ty(actual)?;

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
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let constraint = self.substitute_type(origin.module(), constraint, substitution)?;
            let origin = self.origin_at(origin, source);
            if !answer!(self.constrain_type(origin, Relation::Satisfies, argument, constraint)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
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
    left: SmallVec<[dir::GlobalTypeId; 4]>,
    right: SmallVec<[dir::GlobalTypeId; 4]>,
) -> Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>> {
    if left.len() != right.len() {
        return None;
    }

    Some(left.into_iter().zip(right).collect())
}
