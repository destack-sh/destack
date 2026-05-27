use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericSubstitution, Progress, TermId, TypeRelation, VariableId,
};

/// Function parameter term.
///
/// ```ts
/// value?: string
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionParameterTerm {
    /// The parameter type.
    pub(in crate::check) ty: VariableId,
    /// Whether the parameter may be omitted at the call site.
    pub(in crate::check) is_optional: bool,
    /// Whether the parameter captures the remaining call arguments.
    pub(in crate::check) is_rest: bool,
}

impl FunctionParameterTerm {
    /// Create a required parameter.
    pub(in crate::check) fn required(ty: VariableId) -> Self {
        Self {
            ty,
            is_optional: false,
            is_rest: false,
        }
    }

    /// Substitute generic arguments through this function parameter.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            ty: state.substitute_type_variable(module, substitution, self.ty)?,
            is_optional: self.is_optional,
            is_rest: self.is_rest,
        })
    }
}

/// Function type term.
///
/// ```ts
/// (value: string) => int32
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionTerm {
    /// The function asynchrony.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// The generic parameter types.
    pub(in crate::check) generic_parameters: Vec<VariableId>,
    /// The optional `this` parameter type.
    pub(in crate::check) this_parameter: Option<VariableId>,
    /// The parameter types.
    pub(in crate::check) parameters: Vec<TermId<FunctionParameterTerm>>,
    /// The optional return type.
    pub(in crate::check) return_type: Option<VariableId>,
    /// Whether this is a generator function.
    pub(in crate::check) is_generator: bool,
}

impl FunctionTerm {
    /// Substitute generic arguments through this function term.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            asynchrony: self.asynchrony,
            generic_parameters: self.generic_parameters.clone(),
            this_parameter: self
                .this_parameter
                .map(|parameter| state.substitute_type_variable(module, substitution, parameter))
                .transpose()?,
            parameters: self
                .parameters
                .iter()
                .map(|parameter| {
                    let parameter = state.terms.get(*parameter);
                    let parameter = parameter.substitute(module, substitution, state)?;
                    let parameter = state.terms.push(parameter);

                    Ok(parameter)
                })
                .collect::<CompilerResult<Vec<_>>>()?,
            return_type: self
                .return_type
                .map(|return_type| state.substitute_type_variable(module, substitution, return_type))
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
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.extend(self.generic_parameters.iter().copied());
        variables.extend(self.this_parameter);
        variables.extend(
            self.parameters
                .iter()
                .map(|parameter| state.terms.get(*parameter).ty),
        );
        variables.extend(self.return_type.iter().copied());

        variables
    }
}

impl CheckState<'_> {
    /// Expect one function term to satisfy one expected function type.
    pub(in crate::check) fn expect_function_term(
        &mut self,
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
            progress = progress.merge(self.solve_type_assignability(target, source)?);
        }

        // push parameter context contravariantly
        for (source, target) in function.parameters.iter().zip(&expected.parameters) {
            let source = self.terms.get(*source);
            let target = self.terms.get(*target);

            if source.is_optional != target.is_optional || source.is_rest != target.is_rest {
                continue;
            }

            progress = progress.merge(self.solve_type_assignability(target.ty, source.ty)?);
        }

        // push return context covariantly
        if let (Some(source), Some(target)) = (function.return_type, expected.return_type) {
            progress = progress.merge(self.solve_type_assignability(source, target)?);
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
            self.decide_optional_type_variable_equal(left.this_parameter, right.this_parameter)?;
        let parameters =
            self.decide_function_parameter_list_equal(&left.parameters, &right.parameters)?;
        let return_type =
            self.decide_optional_type_variable_equal(left.return_type, right.return_type)?;

        Ok(generics.and(this).and(parameters).and(return_type))
    }

    /// Decide exact equality for function parameter terms.
    fn decide_function_parameter_list_equal(
        &self,
        left: &[TermId<FunctionParameterTerm>],
        right: &[TermId<FunctionParameterTerm>],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare parameter metadata and types together
        for (left, right) in left.iter().zip(right) {
            let left = self.terms.get(*left);
            let right = self.terms.get(*right);

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
