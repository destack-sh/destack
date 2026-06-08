use destack_dir as dir;

use crate::check::{
    CheckEvent, CheckState, Dump, DumpContext, StaticOperand, StaticTerm, TermId, TraceOperand,
    TypeOperand, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Solved value for one check variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum Solution {
    /// Solved type value.
    Type(TypeSolution),
    /// Solved static value.
    Static(StaticSolution),
}

/// Solved type value for one check variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum TypeSolution {
    /// Check term value.
    Term(TermId<TypeTerm>),
    /// Committed type value.
    Type(dir::GlobalTypeId),
}

/// Solved static value for one check variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum StaticSolution {
    /// Check term value.
    Term(TermId<StaticTerm>),
    /// Committed static value.
    Static(dir::GlobalStaticId),
}

impl From<TypeSolution> for TypeOperand {
    /// Convert a type solution into a type operand.
    fn from(solution: TypeSolution) -> Self {
        match solution {
            TypeSolution::Term(term) => Self::Term(term),
            TypeSolution::Type(ty) => Self::Type(ty),
        }
    }
}

impl From<StaticSolution> for StaticOperand {
    /// Convert a static solution into a static operand.
    fn from(solution: StaticSolution) -> Self {
        match solution {
            StaticSolution::Term(term) => Self::Term(term),
            StaticSolution::Static(value) => Self::Static(value),
        }
    }
}

impl From<TypeSolution> for Solution {
    /// Convert a type solution into a variable solution.
    fn from(solution: TypeSolution) -> Self {
        Self::Type(solution)
    }
}

impl From<StaticSolution> for Solution {
    /// Convert a static solution into a variable solution.
    fn from(solution: StaticSolution) -> Self {
        Self::Static(solution)
    }
}

impl Solution {
    /// Convert this solution into a trace operand.
    pub(in crate::check) fn trace_operand(self) -> TraceOperand {
        match self {
            Self::Type(solution) => TraceOperand::Type(solution.into()),
            Self::Static(solution) => TraceOperand::Static(solution.into()),
        }
    }
}

impl CheckState<'_> {
    /// Set one complete variable solution.
    pub(in crate::check) fn set_variable_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<()> {
        if let Some(existing) = self.inference.variable_solution(variable) {
            let module = variable.module;
            let variable = self.dump_in_module(module, &variable);
            let existing = self.dump_in_module(module, &existing);
            let solution = self.dump_in_module(module, &solution);

            return Err(CompilerError::Internal {
                message: format!(
                    "check variable {variable} already has solution {existing}, got {solution}"
                ),
            });
        }

        self.inference.set_variable_solution(variable, solution)?;
        self.inference.wake_variable(variable);

        self.record_event(CheckEvent::SolutionSet {
            variable,
            kind: self.variable(variable).kind,
            value: solution.trace_operand(),
        });

        Ok(())
    }

    /// Return the solved type operand for one variable.
    pub(in crate::check) fn variable_type_solution_operand(
        &self,
        variable: VariableId,
    ) -> Option<TypeOperand> {
        match self.inference.variable_solution(variable) {
            Some(Solution::Type(solution)) => Some(solution.into()),
            Some(Solution::Static(_)) | None => None,
        }
    }

    /// Return the solved static operand for one variable.
    pub(in crate::check) fn variable_static_solution_operand(
        &self,
        variable: VariableId,
    ) -> Option<StaticOperand> {
        match self.inference.variable_solution(variable) {
            Some(Solution::Static(solution)) => Some(solution.into()),
            Some(Solution::Type(_)) | None => None,
        }
    }
}

impl Dump for Solution {
    /// Render this solution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Type(solution) => TypeOperand::from(*solution).dump(context),
            Self::Static(solution) => StaticOperand::from(*solution).dump(context),
        }
    }
}
