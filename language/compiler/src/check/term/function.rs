use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, Origin, Progress, ReceiverSubstitution, Substitution, TypeOperand,
    TypeRelation, VariableId,
};

/// Function parameter payload.
///
/// ```ds
/// value?: string
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionParameter {
    /// The parameter type.
    pub(in crate::check) ty: TypeOperand,
    /// The static generic slot supplied by this runtime argument.
    pub(in crate::check) static_slot: Option<Box<VariableId>>,
    /// Whether the parameter may be omitted at the call site.
    pub(in crate::check) is_optional: bool,
    /// Whether the parameter captures the remaining call arguments.
    pub(in crate::check) is_rest: bool,
}

impl FunctionParameter {
    /// Create a required parameter.
    pub(in crate::check) fn required(ty: impl Into<TypeOperand>) -> Self {
        Self {
            ty: ty.into(),
            static_slot: None,
            is_optional: false,
            is_rest: false,
        }
    }

    /// Substitute generic arguments through this function parameter.
    pub(in crate::check) fn substitute<'a>(
        &self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            ty: state.substitute_type_operand(module, substitution, self.ty)?,
            static_slot: None,
            is_optional: self.is_optional,
            is_rest: self.is_rest,
        })
    }

    /// Substitute the selected receiver through this function parameter.
    pub(in crate::check) fn substitute_receiver(
        &self,
        module: ModuleId,
        substitution: ReceiverSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            ty: state.substitute_receiver_type_operand(module, self.ty, substitution)?,
            static_slot: self.static_slot.clone(),
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
    /// The generic parameter types.
    pub(in crate::check) generic_parameters: Vec<VariableId>,
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
    /// Substitute generic arguments through this function term.
    pub(in crate::check) fn substitute<'a>(
        &self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let mut generic_parameters = Vec::new();
        let substitution = substitution.into();
        if let Some(generic) = substitution.generic {
            for parameter in &self.generic_parameters {
                let slot = state.generic_parameter_slot(*parameter).id();
                let is_substituted = generic.entries.iter().any(|entry| entry.slot == slot);
                if !is_substituted {
                    generic_parameters.push(*parameter);
                }
            }
        } else {
            generic_parameters.extend(self.generic_parameters.iter().copied());
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

    /// Substitute the selected receiver through this function term.
    pub(in crate::check) fn substitute_receiver(
        &self,
        module: ModuleId,
        substitution: ReceiverSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            asynchrony: self.asynchrony,
            generic_parameters: self.generic_parameters.clone(),
            this_parameter: self
                .this_parameter
                .map(|parameter| {
                    state.substitute_receiver_type_operand(module, parameter, substitution)
                })
                .transpose()?,
            parameters: self
                .parameters
                .iter()
                .map(|parameter| parameter.substitute_receiver(module, substitution, state))
                .collect::<CompilerResult<SmallVec<_>>>()?,
            return_type: self
                .return_type
                .map(|return_type| {
                    state.substitute_receiver_type_operand(module, return_type, substitution)
                })
                .transpose()?,
            is_generator: self.is_generator,
        })
    }
}

impl FunctionTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        variables.extend(self.generic_parameters.iter().copied());
        variables.extend(
            self.this_parameter
                .into_iter()
                .flat_map(|parameter| parameter.referenced_variables(state)),
        );
        for parameter in &self.parameters {
            variables.extend(parameter.ty.referenced_variables(state));
        }
        variables.extend(
            self.return_type
                .into_iter()
                .flat_map(|return_type| return_type.referenced_variables(state)),
        );

        variables
    }
}

impl CheckState<'_> {
    /// Return a function term from one committed DIR function type.
    pub(in crate::check) fn function_type_term(
        &mut self,
        module: ModuleId,
        function: dir::FunctionType,
    ) -> CompilerResult<FunctionTerm> {
        let mut generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for parameter in function.generic_parameters {
            let parameter = self.global_type_operand(module, parameter);
            let TypeOperand::Variable(parameter) = parameter else {
                panic!("committed function generic parameter is not a check variable");
            };

            generic_parameters.push(parameter);
        }

        let this_parameter = function
            .this_parameter
            .map(|parameter| self.global_type_operand(module, parameter));
        let parameters = function
            .parameters
            .into_iter()
            .map(|parameter| {
                let static_slot = parameter
                    .static_slot
                    .map(|slot| Box::new(self.generic_parameter_variable(module, slot)));

                FunctionParameter {
                    ty: self.global_type_operand(module, parameter.ty),
                    static_slot,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                }
            })
            .collect();
        let return_type = function
            .return_type
            .map(|return_type| self.global_type_operand(module, return_type));

        Ok(FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: function.is_generator,
        })
    }

    /// Expect one function term to satisfy one expected function type.
    pub(in crate::check) fn expect_function_term(
        &mut self,
        origin: Origin,
        function: &FunctionTerm,
        expected: &FunctionTerm,
    ) -> CompilerResult<Progress> {
        if function.asynchrony != expected.asynchrony
            || function.is_generator != expected.is_generator
        {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // push receiver context into the function input
        if let (Some(source), Some(target)) = (function.this_parameter, expected.this_parameter) {
            progress =
                progress.merge(self.relate_contextual_type_assignability(origin, target, source)?);
        }

        // push parameter context contravariantly
        for (source, target) in function.parameters.iter().zip(&expected.parameters) {
            if source.is_optional != target.is_optional || source.is_rest != target.is_rest {
                continue;
            }

            progress = progress
                .merge(self.relate_contextual_type_assignability(origin, target.ty, source.ty)?);
        }

        // push return context covariantly
        if let (Some(source), Some(target)) = (function.return_type, expected.return_type) {
            progress =
                progress.merge(self.relate_contextual_type_assignability(origin, source, target)?);
        }

        Ok(progress)
    }

    /// Decide exact equality for function terms.
    pub(in crate::check) fn decide_function_equal(
        &self,
        left: &FunctionTerm,
        right: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if left.asynchrony != right.asynchrony || left.is_generator != right.is_generator {
            return Ok(Decision::No);
        }

        let generics = self
            .decide_type_variable_list_equal(&left.generic_parameters, &right.generic_parameters)?;
        let this =
            self.decide_optional_type_operand_equal(left.this_parameter, right.this_parameter)?;
        let parameters =
            self.decide_function_parameter_list_equal(&left.parameters, &right.parameters)?;
        let return_type =
            self.decide_optional_type_operand_equal(left.return_type, right.return_type)?;

        Ok(generics.and(this).and(parameters).and(return_type))
    }

    /// Decide exact equality for function parameter terms.
    fn decide_function_parameter_list_equal(
        &self,
        left: &[FunctionParameter],
        right: &[FunctionParameter],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare parameter metadata and types together
        for (left, right) in left.iter().zip(right) {
            if left.is_optional != right.is_optional || left.is_rest != right.is_rest {
                return Ok(Decision::No);
            }
            decision =
                decision.and(self.decide_type_relation(TypeRelation::Equal, left.ty, right.ty)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
