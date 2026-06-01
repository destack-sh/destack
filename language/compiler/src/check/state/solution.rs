use destack_dir as dir;

use crate::check::{
    CheckState, StaticOperand, StaticTerm, TermId, TypeOperand, TypeTerm, VariableId,
};

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
    /// Committed DIR type value.
    Type(dir::GlobalTypeId),
}

/// Solved static value for one check variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum StaticSolution {
    /// Check term value.
    Term(TermId<StaticTerm>),
    /// Committed DIR static value.
    Static(dir::GlobalStaticId),
}

impl TryFrom<TypeOperand> for TypeSolution {
    type Error = VariableId;

    /// Convert a non-variable type operand into a type solution.
    fn try_from(operand: TypeOperand) -> Result<Self, Self::Error> {
        match operand {
            TypeOperand::Variable(variable) => Err(variable),
            TypeOperand::Term(term) => Ok(Self::Term(term)),
            TypeOperand::Type(ty) => Ok(Self::Type(ty)),
        }
    }
}

impl TryFrom<StaticOperand> for StaticSolution {
    type Error = VariableId;

    /// Convert a non-variable static operand into a static solution.
    fn try_from(operand: StaticOperand) -> Result<Self, Self::Error> {
        match operand {
            StaticOperand::Variable(variable) => Err(variable),
            StaticOperand::Term(term) => Ok(Self::Term(term)),
            StaticOperand::Static(value) => Ok(Self::Static(value)),
        }
    }
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

impl CheckState<'_> {
    /// Insert one solution known before ordinary solver reduction.
    pub(in crate::check) fn insert_known_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) {
        let previous = self.inference.insert_variable_solution(variable, solution);

        assert!(
            previous.is_none(),
            "check variable {variable:?} already has a known solution"
        );
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
