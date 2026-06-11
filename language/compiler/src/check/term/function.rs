use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, GenericParameterId, Origin, SubstitutionSet, TermId,
    TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
};

/// Function parameter payload.
///
/// ```ds
/// value?: string
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct FunctionParameter {
    /// The parameter type.
    pub(in crate::check) ty: TypeOperand,
    /// The static generic parameter supplied by this runtime argument.
    pub(in crate::check) static_parameter: Option<GenericParameterId>,
    /// Whether the parameter type came from inference.
    pub(in crate::check) is_inferred: bool,
    /// Whether the parameter may be omitted at the call site.
    pub(in crate::check) is_optional: bool,
    /// Whether the parameter captures the remaining call arguments.
    pub(in crate::check) is_rest: bool,
}

impl FunctionParameter {
    /// Substitute generic arguments through this function parameter.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let static_parameter = if substitution.has_generics() {
            None
        } else {
            self.static_parameter
        };

        Ok(Self {
            ty: state.substitute_type_operand(module, substitution, self.ty)?,
            static_parameter,
            is_inferred: self.is_inferred,
            is_optional: self.is_optional,
            is_rest: self.is_rest,
        })
    }
}

/// Function type term.
///
/// ```ds
/// (value: string) => int32
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionTerm {
    /// The function asynchrony.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// The generic parameter parameters.
    pub(in crate::check) generic_parameters: SmallVec<[GenericParameterId; 2]>,
    /// The optional `this` parameter type.
    pub(in crate::check) this_parameter: Option<TypeOperand>,
    /// The parameter types.
    pub(in crate::check) parameters: SmallVec<[FunctionParameter; 2]>,
    /// The optional return type.
    pub(in crate::check) return_type: Option<TypeOperand>,
    /// Whether this is a generator function.
    pub(in crate::check) is_generator: bool,
}

impl FunctionTerm {
    /// Return whether source parameters can receive contextual target arities.
    pub(in crate::check) fn accepts_contextual_arities(
        source: &[FunctionParameter],
        target: &[FunctionParameter],
    ) -> bool {
        let source_range = Self::parameter_count_range(source, true);
        let target_range = Self::parameter_count_range(target, false);

        source_range.accepts(target_range)
    }

    /// Return the accepted arity range for one parameter sequence.
    fn parameter_count_range(
        parameters: &[FunctionParameter],
        contextual: bool,
    ) -> FunctionParameterCountRange {
        let required = parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .filter(|parameter| !contextual || !parameter.is_inferred)
            .count();

        FunctionParameterCountRange { required }
    }
}

impl FunctionParameterCountRange {
    /// Return whether this range accepts every target call arity.
    fn accepts(self, target_range: Self) -> bool {
        self.required <= target_range.required
    }
}

impl FunctionTerm {
    /// Substitute generic arguments through this function term.
    pub(in crate::check) fn substitute<'a>(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let mut generic_parameters = SmallVec::<[GenericParameterId; 2]>::new();

        // retain generic parameters that are not applied by this substitution
        for parameter in &self.generic_parameters {
            if !substitution.has_generic(*parameter) {
                generic_parameters.push(*parameter);
            }
        }

        Ok(Self {
            asynchrony: self.asynchrony,
            generic_parameters,
            this_parameter: self
                .this_parameter
                .map(|parameter| state.substitute_type_operand(module, substitution, parameter))
                .transpose()?,
            parameters: self
                .parameters
                .iter()
                .map(|parameter| parameter.substitute(module, substitution, state))
                .collect::<CompilerResult<SmallVec<_>>>()?,
            return_type: self
                .return_type
                .map(|return_type| state.substitute_type_operand(module, substitution, return_type))
                .transpose()?,
            is_generator: self.is_generator,
        })
    }
}

/// Function parameter count range.
struct FunctionParameterCountRange {
    /// The smallest accepted runtime argument count.
    required: usize,
}

impl CheckState<'_> {
    /// Check one function term to satisfy one expected function type.
    pub(in crate::check) fn expect_function_term(
        &mut self,
        origin: Origin,
        function: TermId<FunctionTerm>,
        expected: TermId<FunctionTerm>,
    ) -> CompilerResult<()> {
        let function_id = function;
        let expected_id = expected;
        let function = self.inference.term(function_id);
        let function_asynchrony = function.asynchrony;
        let function_is_generator = function.is_generator;
        let function_this = function.this_parameter;
        let function_len = function.parameters.len();
        let function_return = function.return_type;

        let expected = self.inference.term(expected_id);
        let expected_asynchrony = expected.asynchrony;
        let expected_is_generator = expected.is_generator;
        let expected_this = expected.this_parameter;
        let expected_len = expected.parameters.len();
        let expected_return = expected.return_type;

        if function_asynchrony != expected_asynchrony
            || function_is_generator != expected_is_generator
        {
            return Ok(());
        }

        // push receiver context into the function input
        if let (Some(source), Some(target)) = (function_this, expected_this) {
            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                target,
                source,
                Condition::Always,
            );
        }

        // push parameter context contravariantly
        if !self.function_accepts_contextual_arities(function_id, expected_id) {
            return Ok(());
        }
        for index in 0..function_len.min(expected_len) {
            let source = self.inference.term(function_id).parameters[index];
            let target = self.inference.term(expected_id).parameters[index];

            if source.is_rest != target.is_rest {
                continue;
            }
            let relation = if source.is_inferred {
                TypeRelation::Equal
            } else {
                TypeRelation::Assignable
            };

            self.constrain_type(origin, relation, target.ty, source.ty, Condition::Always);
        }

        // push return context covariantly
        match (function_return, expected_return) {
            (Some(source), Some(target)) => {
                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    source,
                    target,
                    Condition::Always,
                );
            }
            (None, Some(target)) => {
                let source = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Void));

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    source,
                    target,
                    Condition::Always,
                );
            }
            (Some(_), None) | (None, None) => {}
        }

        Ok(())
    }

    /// Decide exact equality for function terms.
    pub(in crate::check) fn decide_function_equal(
        &mut self,
        left: TermId<FunctionTerm>,
        right: TermId<FunctionTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let left_id = left;
        let right_id = right;
        let left = self.inference.term(left_id);
        let left_asynchrony = left.asynchrony;
        let left_is_generator = left.is_generator;
        let left_this = left.this_parameter;
        let left_return = left.return_type;

        let right = self.inference.term(right_id);
        let right_asynchrony = right.asynchrony;
        let right_is_generator = right.is_generator;
        let right_this = right.this_parameter;
        let right_return = right.return_type;

        if left_asynchrony != right_asynchrony || left_is_generator != right_is_generator {
            return Ok(Answer::Ready(false));
        }

        let generics = if self.function_generic_parameters_equal(left_id, right_id) {
            Answer::Ready(true)
        } else {
            Answer::Ready(false)
        };
        let this = self.decide_optional_type_operand_equal(left_this, right_this)?;
        let parameters = self.decide_function_parameters_equal(left_id, right_id)?;
        let return_type = self.decide_optional_type_operand_equal(left_return, right_return)?;

        Ok(generics.and(this).and(parameters).and(return_type))
    }

    /// Decide whether one function type is assignable to another.
    pub(in crate::check) fn decide_function_assignable(
        &mut self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let source_id = source;
        let target_id = target;
        let source = self.inference.term(source_id);
        let source_asynchrony = source.asynchrony;
        let source_is_generator = source.is_generator;
        let source_this = source.this_parameter;
        let source_return = source.return_type;

        let target = self.inference.term(target_id);
        let target_asynchrony = target.asynchrony;
        let target_is_generator = target.is_generator;
        let target_this = target.this_parameter;
        let target_return = target.return_type;

        if source_asynchrony != target_asynchrony || source_is_generator != target_is_generator {
            return Ok(Answer::Ready(false));
        }

        // compare receiver input contravariantly
        let receiver = self.decide_function_receiver_assignable(source_this, target_this)?;
        if receiver == Answer::Ready(false) {
            return Ok(receiver);
        }

        // compare runtime inputs contravariantly
        let parameters = self.decide_function_parameters_assignable(source_id, target_id)?;
        if parameters == Answer::Ready(false) {
            return Ok(parameters);
        }

        // compare outputs covariantly
        let return_type = self.decide_function_return_assignable(source_return, target_return)?;

        Ok(receiver.and(parameters).and(return_type))
    }

    /// Decide whether one function receiver can accept another receiver.
    fn decide_function_receiver_assignable(
        &mut self,
        source: Option<TypeOperand>,
        target: Option<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (source, target) {
            (Some(source), Some(target)) => {
                self.decide_type_relation(TypeRelation::Assignable, target, source)?
            }
            (None, _) => Answer::Ready(true),
            (Some(_), None) => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Return whether source parameters can receive contextual target arities.
    fn function_accepts_contextual_arities(
        &self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> bool {
        let source = &self.inference.term(source).parameters;
        let target = &self.inference.term(target).parameters;

        FunctionTerm::accepts_contextual_arities(source, target)
    }

    /// Return whether two function generic parameter lists match.
    fn function_generic_parameters_equal(
        &self,
        left: TermId<FunctionTerm>,
        right: TermId<FunctionTerm>,
    ) -> bool {
        self.inference.term(left).generic_parameters
            == self.inference.term(right).generic_parameters
    }

    /// Decide whether source parameters accept every target call.
    fn decide_function_parameters_assignable(
        &mut self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> CompilerResult<Answer<bool>> {
        if !self.function_accepts_contextual_arities(source, target) {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);
        let source_len = self.inference.term(source).parameters.len();
        let target_len = self.inference.term(target).parameters.len();

        // compare parameter metadata and types together
        for index in 0..source_len.min(target_len) {
            let source = self.inference.term(source).parameters[index];
            let target = self.inference.term(target).parameters[index];

            if source.is_rest != target.is_rest {
                return Ok(Answer::Ready(false));
            }
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                target.ty,
                source.ty,
            )?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source return type satisfies one target return type.
    fn decide_function_return_assignable(
        &mut self,
        source: Option<TypeOperand>,
        target: Option<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (source, target) {
            (Some(source), Some(target)) => {
                self.decide_type_relation(TypeRelation::Assignable, source, target)?
            }
            (Some(_), None) => Answer::Ready(true),
            (None, Some(target)) => {
                let void = self
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Void));

                self.decide_type_relation(TypeRelation::Assignable, void, target)?
            }
            (None, None) => Answer::Ready(true),
        };

        Ok(decision)
    }

    /// Decide exact equality for function parameters.
    fn decide_function_parameters_equal(
        &mut self,
        left: TermId<FunctionTerm>,
        right: TermId<FunctionTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let left_len = self.inference.term(left).parameters.len();
        let right_len = self.inference.term(right).parameters.len();
        if left_len != right_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // compare parameter metadata and types together
        for index in 0..left_len {
            let left = self.inference.term(left).parameters[index];
            let right = self.inference.term(right).parameters[index];

            if left.is_optional != right.is_optional || left.is_rest != right.is_rest {
                return Ok(Answer::Ready(false));
            }
            decision =
                decision.and(self.decide_type_relation(TypeRelation::Equal, left.ty, right.ty)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
