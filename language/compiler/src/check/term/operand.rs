use std::fmt::{Debug, Formatter};

use smallvec::SmallVec;

use crate::check::{CheckState, StaticTerm, Term, TermId, TypeTerm, VariableId};

/// Relation operand in one term space.
pub(in crate::check) enum Operand<T: Term> {
    /// A solver variable.
    Variable(VariableId),
    /// A fixed term.
    Term(TermId<T>),
}

/// Type relation operand.
pub(in crate::check) type TypeOperand = Operand<TypeTerm>;

/// Static relation operand.
pub(in crate::check) type StaticOperand = Operand<StaticTerm>;

impl<T: Term> Operand<T> {
    /// Return the variable identity when this operand has one.
    pub(in crate::check) fn variable(self) -> Option<VariableId> {
        match self {
            Self::Variable(variable) => Some(variable),
            Self::Term(_) => None,
        }
    }
}

impl<T: Term> Clone for Operand<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Term> Copy for Operand<T> {}

impl<T: Term> PartialEq for Operand<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Variable(left), Self::Variable(right)) => left == right,
            (Self::Term(left), Self::Term(right)) => left.id == right.id,
            (Self::Variable(_), Self::Term(_)) | (Self::Term(_), Self::Variable(_)) => false,
        }
    }
}

impl<T: Term> Eq for Operand<T> {}

impl<T: Term> Debug for Operand<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(variable) => f.debug_tuple("Variable").field(variable).finish(),
            Self::Term(term) => f.debug_tuple("Term").field(&term.id).finish(),
        }
    }
}

impl Operand<TypeTerm> {
    /// Return this operand as a type term.
    pub(in crate::check) fn to_type_term(self, state: &CheckState<'_>) -> TypeTerm {
        match self {
            Self::Variable(variable) => TypeTerm::Variable(variable),
            Self::Term(term) => state.terms.get(term).clone(),
        }
    }

    /// Return variables referenced by this operand.
    pub(in crate::check) fn referenced_variables(
        self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Variable(variable) => smallvec::smallvec![variable],
            Self::Term(term) => state.terms.get(term).referenced_variables(state),
        }
    }
}

impl Operand<StaticTerm> {
    /// Return variables referenced by this operand.
    pub(in crate::check) fn referenced_variables(
        self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Variable(variable) => smallvec::smallvec![variable],
            Self::Term(term) => state.terms.get(term).referenced_variables(state),
        }
    }
}

impl From<VariableId> for Operand<TypeTerm> {
    fn from(variable: VariableId) -> Self {
        Self::Variable(variable)
    }
}

impl From<TermId<TypeTerm>> for Operand<TypeTerm> {
    fn from(term: TermId<TypeTerm>) -> Self {
        Self::Term(term)
    }
}

impl From<VariableId> for Operand<StaticTerm> {
    fn from(variable: VariableId) -> Self {
        Self::Variable(variable)
    }
}

impl From<TermId<StaticTerm>> for Operand<StaticTerm> {
    fn from(term: TermId<StaticTerm>) -> Self {
        Self::Term(term)
    }
}
