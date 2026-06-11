use destack_dir as dir;

use crate::check::{StaticOperand, StaticTerm, TermId, TraceOperand, TypeOperand, TypeTerm};

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

impl TryFrom<TypeOperand> for TypeSolution {
    type Error = ();

    /// Convert a non-variable type operand into a type solution.
    fn try_from(operand: TypeOperand) -> Result<Self, Self::Error> {
        match operand {
            TypeOperand::Term(term) => Ok(Self::Term(term)),
            TypeOperand::Type(ty) => Ok(Self::Type(ty)),
            TypeOperand::Variable(_) => Err(()),
        }
    }
}

impl TryFrom<StaticOperand> for StaticSolution {
    type Error = ();

    /// Convert a non-variable static operand into a static solution.
    fn try_from(operand: StaticOperand) -> Result<Self, Self::Error> {
        match operand {
            StaticOperand::Term(term) => Ok(Self::Term(term)),
            StaticOperand::Static(value) => Ok(Self::Static(value)),
            StaticOperand::Variable(_) => Err(()),
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
