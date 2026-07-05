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

/// Direct bindings found by matching a generic type pattern.
#[derive(Debug)]
struct GenericMatch {
    /// The matched parameters in declaration order.
    parameters: SmallVec<[GenericParameterId; 4]>,
    /// The matched arguments in declaration order.
    arguments: SmallVec<[Option<dir::GlobalTypeId>; 4]>,
}

impl GenericMatch {
    /// Create an empty match for one ordered parameter list.
    fn new(parameters: SmallVec<[GenericParameterId; 4]>) -> Self {
        let arguments = parameters.iter().map(|_| None).collect();

        Self {
            parameters,
            arguments,
        }
    }

    /// Return the parameter binding position.
    fn parameter_index(&self, parameter: GenericParameterId) -> Option<usize> {
        self.parameters
            .iter()
            .position(|candidate| *candidate == parameter)
    }

    /// Bind one parameter to one concrete argument.
    fn bind(
        &mut self,
        state: &mut CheckState<'_>,
        origin: Origin,
        parameter: GenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let Some(index) = self.parameter_index(parameter) else {
            return Ok(Answer::Ready(false));
        };

        // bind the first occurrence directly
        let Some(bound) = self.arguments[index] else {
            self.arguments[index] = Some(argument);

            return Ok(Answer::Ready(true));
        };

        // repeated occurrences must agree
        state.decide_relation(origin, Relation::Equal, bound, argument)
    }
}

impl CheckState<'_> {
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

            if !answer!(self.constrain_generic_bound(origin, argument_source, argument, bound,)?) {
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
        let mut generic = GenericMatch::new(parameters.iter().copied().collect());
        if !answer!(self.match_generic_type(origin, &mut generic, pattern, actual)?) {
            return Ok(Answer::Ready(None));
        }

        let source = self.origin_source_node(origin)?;
        let substitution = self.generic_match_substitution(generic)?;
        if !answer!(self.check_generic_substitution_bounds(
            origin,
            source.into_global(origin.module()),
            &substitution,
        )?) {
            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Match one parameter list over positional pattern/actual pairs.
    ///
    /// Parameters the pairs leave free self-bind and stay rigid, and
    /// bound arguments must satisfy their declared constraints.
    pub(in crate::check) fn match_generic_pairs(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        let mut generic = GenericMatch::new(parameters.iter().copied().collect());
        for (pattern, actual) in pairs.iter().copied() {
            if !answer!(self.match_generic_type(origin, &mut generic, pattern, actual)?) {
                return Ok(Answer::Ready(None));
            }
        }

        let source = self.origin_source_node(origin)?;
        let substitution = self.generic_match_substitution(generic)?;
        if !answer!(self.check_generic_substitution_bounds(
            origin,
            source.into_global(origin.module()),
            &substitution,
        )?) {
            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Match one generic type pattern without opening inference variables.
    fn match_generic_type(
        &mut self,
        origin: Origin,
        generic: &mut GenericMatch,
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let pattern = answer!(self.reduce_type(origin, pattern)?);
        let actual = answer!(self.reduce_type(origin, actual)?);

        let pattern_type = self.ty(pattern)?;
        let actual_type = self.ty(actual)?;

        // bind template parameters directly
        if let dir::Type::Parameter(parameter) = pattern_type
            && generic.parameter_index(parameter).is_some()
        {
            return generic.bind(self, origin, parameter, actual);
        }

        // identical types still decompose while the pattern mentions
        // bindable parameters, so identity matches record their bindings
        if pattern == actual && !self.type_flags(pattern)?.has_parameter() {
            return Ok(Answer::Ready(true));
        }

        // decompose fixed slots beneath one shared constructor
        let pairs = self.decompose_type_pair(pattern, actual)?;
        if let Some(pairs) = pairs {
            return self.match_generic_arguments(origin, generic, &pairs);
        }

        // childless constructors must agree exactly
        Ok(Answer::Ready(pattern_type == actual_type))
    }

    /// Return the substitution captured by one direct generic match.
    fn generic_match_substitution(
        &mut self,
        generic: GenericMatch,
    ) -> CompilerResult<TypeSubstitution> {
        let mut parameters = SmallVec::<[GenericParameterId; 4]>::new();
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // collect matched parameters; parameters the pattern leaves
        // free stay absent, so callers may still open them later
        for (index, parameter) in generic.parameters.iter().copied().enumerate() {
            let Some(argument) = generic.arguments[index] else {
                continue;
            };

            parameters.push(parameter);
            arguments.push(argument);
        }

        Ok(TypeSubstitution {
            parameters,
            arguments,
            receiver: None,
        })
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
            if !answer!(self.constrain_generic_bound(origin, source, argument, constraint)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
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

    /// Match fixed positional type pairs.
    fn match_generic_arguments(
        &mut self,
        origin: Origin,
        generic: &mut GenericMatch,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pairs.iter().copied() {
            decision = decision.and(self.match_generic_type(origin, generic, pattern, actual)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
