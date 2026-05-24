use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckComponentState, Decision, GenericSubstitution, VariableId};

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
    pub(in crate::check) parameters: Vec<VariableId>,
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
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            asynchrony: self.asynchrony,
            generic_parameters: self.generic_parameters.clone(),
            this_parameter: self
                .this_parameter
                .map(|parameter| state.substitute_type_variable(module, substitution, parameter))
                .transpose()?,
            parameters: state.substitute_type_variables(module, substitution, &self.parameters)?,
            return_type: self
                .return_type
                .map(|return_type| {
                    state.substitute_type_variable(module, substitution, return_type)
                })
                .transpose()?,
            is_generator: self.is_generator,
        })
    }
}

impl FunctionTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.extend(self.generic_parameters.iter().copied());
        variables.extend(self.this_parameter);
        variables.extend(self.parameters.iter().copied());
        variables.extend(self.return_type.iter().copied());

        variables
    }
}

impl CheckComponentState<'_> {
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
            self.decide_type_variable_list_equal(&left.parameters, &right.parameters)?;
        let return_type =
            self.decide_optional_type_variable_equal(left.return_type, right.return_type)?;

        Ok(generics.and(this).and(parameters).and(return_type))
    }
}
