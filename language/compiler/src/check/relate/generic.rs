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
        let substitution = self.generic_match_substitution(origin, generic)?;
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

        if pattern == actual {
            return Ok(Answer::Ready(true));
        }

        self.match_generic_type_inner(
            origin,
            generic,
            pattern.module_id,
            pattern_type,
            actual.module_id,
            actual_type,
        )
    }

    /// Return the substitution captured by one direct generic match.
    fn generic_match_substitution(
        &mut self,
        origin: Origin,
        generic: GenericMatch,
    ) -> CompilerResult<TypeSubstitution> {
        let mut parameters = SmallVec::<[GenericParameterId; 4]>::new();
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // collect matched parameters and closed defaults
        for (index, parameter) in generic.parameters.iter().copied().enumerate() {
            let argument = match generic.arguments[index] {
                Some(argument) => argument,
                None => {
                    let Some(default) = self.generic_parameter_default(
                        origin.module(),
                        parameter,
                        &parameters,
                        &arguments,
                    )?
                    else {
                        continue;
                    };

                    default
                }
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

    /// Match two type constructors after roots have been reduced.
    fn match_generic_type_inner(
        &mut self,
        origin: Origin,
        generic: &mut GenericMatch,
        pattern_module: destack_source::ModuleId,
        pattern: dir::Type,
        actual_module: destack_source::ModuleId,
        actual: dir::Type,
    ) -> CompilerResult<Answer<bool>> {
        match (pattern, actual) {
            // nominal applications match by declaration and argument position
            (dir::Type::Instance(pattern), dir::Type::Instance(actual))
                if pattern.symbol == actual.symbol =>
            {
                let pattern_arguments = self.type_ids(pattern_module, pattern.arguments)?.to_vec();
                let actual_arguments = self.type_ids(actual_module, actual.arguments)?.to_vec();

                self.match_generic_arguments(origin, generic, &pattern_arguments, &actual_arguments)
            }

            // value containers match by their contained type
            (dir::Type::Array(pattern), dir::Type::Array(actual)) => {
                self.match_generic_type(origin, generic, pattern.element, actual.element)
            }
            (dir::Type::Slice(pattern), dir::Type::Slice(actual)) => {
                self.match_generic_type(origin, generic, pattern.element, actual.element)
            }
            (dir::Type::FixedArray(pattern), dir::Type::FixedArray(actual)) => {
                let pattern = [pattern.element, pattern.count];
                let actual = [actual.element, actual.count];

                self.match_generic_arguments(origin, generic, &pattern, &actual)
            }

            // tuples match element by element
            (dir::Type::Tuple(pattern), dir::Type::Tuple(actual))
                if pattern.elements.len() == actual.elements.len() =>
            {
                let pattern = self
                    .tuple_elements(pattern_module, pattern.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect::<SmallVec<[_; 4]>>();
                let actual = self
                    .tuple_elements(actual_module, actual.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect::<SmallVec<[_; 4]>>();

                self.match_generic_arguments(origin, generic, &pattern, &actual)
            }

            // memory forms match by form and payload
            (dir::Type::Form(pattern), dir::Type::Form(actual)) if pattern.form == actual.form => {
                self.match_generic_type(origin, generic, pattern.value, actual.value)
            }

            // non-generic leaves must match exactly
            (pattern, actual) if pattern == actual => Ok(Answer::Ready(true)),
            _ => Ok(Answer::Ready(false)),
        }
    }

    /// Match positional type arguments.
    fn match_generic_arguments(
        &mut self,
        origin: Origin,
        generic: &mut GenericMatch,
        pattern: &[dir::GlobalTypeId],
        actual: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if pattern.len() != actual.len() {
            return Ok(Answer::Ready(false));
        }

        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pattern.iter().copied().zip(actual.iter().copied()) {
            decision = decision.and(self.match_generic_type(origin, generic, pattern, actual)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
