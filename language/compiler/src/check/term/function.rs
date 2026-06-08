use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CheckState, Decision, GenericParameterId, Origin, SubstitutionSet, TypeOperand, TypeRelation,
    VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Function parameter payload.
///
/// ```ds
/// value?: string
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionParameter {
    /// The parameter type.
    pub(in crate::check) ty: TypeOperand,
    /// The static generic parameter supplied by this runtime argument.
    pub(in crate::check) static_parameter: Option<GenericParameterId>,
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

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

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
    /// Return a function term from one committed function type.
    pub(in crate::check) fn function_type_term(
        &mut self,
        module: ModuleId,
        function: dir::FunctionType,
    ) -> CompilerResult<FunctionTerm> {
        let mut generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for parameter in function.generic_parameters {
            let dir::Type::Parameter(parameter) = self.r#type(parameter) else {
                return Err(CompilerError::Internal {
                    message: "committed function generic parameter is not a parameter type"
                        .to_owned(),
                });
            };

            generic_parameters.push(self.import_generic_parameter_id(module, *parameter)?);
        }

        let this_parameter = function
            .this_parameter
            .map(|parameter| self.import_type_operand(module, parameter))
            .transpose()?;
        let mut parameters = Vec::with_capacity(function.parameters.len());
        for parameter in function.parameters {
            let static_parameter = parameter
                .static_parameter
                .map(|parameter| self.import_generic_parameter_id(module, parameter))
                .transpose()?;

            parameters.push(FunctionParameter {
                ty: self.import_type_operand(module, parameter.ty)?,
                static_parameter,
                is_optional: parameter.is_optional,
                is_rest: parameter.is_rest,
            });
        }
        let return_type = function
            .return_type
            .map(|return_type| self.import_type_operand(module, return_type))
            .transpose()?;

        Ok(FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters: generic_parameters.into(),
            this_parameter,
            parameters: parameters.into(),
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
    ) -> CompilerResult<()> {
        if function.asynchrony != expected.asynchrony
            || function.is_generator != expected.is_generator
        {
            return Ok(());
        }

        // push receiver context into the function input
        if let (Some(source), Some(target)) = (function.this_parameter, expected.this_parameter) {
            self.reduce_contextual_type_assignability(origin, target, source)?;
        }

        // push parameter context contravariantly
        for (source, target) in function.parameters.iter().zip(&expected.parameters) {
            if source.is_optional != target.is_optional || source.is_rest != target.is_rest {
                continue;
            }

            self.reduce_contextual_type_assignability(origin, target.ty, source.ty)?;
        }

        // push return context covariantly
        if let (Some(source), Some(target)) = (function.return_type, expected.return_type) {
            self.reduce_contextual_type_assignability(origin, source, target)?;
        }

        Ok(())
    }

    /// Decide exact equality for function terms.
    pub(in crate::check) fn decide_function_equal(
        &mut self,
        left: &FunctionTerm,
        right: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if left.asynchrony != right.asynchrony || left.is_generator != right.is_generator {
            return Ok(Decision::No);
        }

        let generics = if left.generic_parameters == right.generic_parameters {
            Decision::Yes
        } else {
            Decision::No
        };
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
        &mut self,
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
