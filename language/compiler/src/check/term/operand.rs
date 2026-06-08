use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{CheckState, StaticTerm, TermId, TypeTerm, VariableId};

/// Type relation operand.
pub(in crate::check) enum TypeOperand {
    /// A solver variable.
    ///
    /// Examples:
    /// ```ds
    /// const inferred = value
    /// ```
    Variable(VariableId),
    /// A fixed check term.
    ///
    /// Examples:
    /// ```ds
    /// const known: string = value
    /// ```
    Term(TermId<TypeTerm>),
    /// A committed type.
    ///
    /// Examples:
    /// ```ds
    /// import { Value } from "./dependency"
    /// ```
    Type(dir::GlobalTypeId),
}

// assert that TypeOperand <= 64B
const _: () = assert!(std::mem::size_of::<TypeOperand>() <= 64);

/// Static relation operand.
pub(in crate::check) enum StaticOperand {
    /// A solver variable.
    ///
    /// Examples:
    /// ```ds
    /// <comptime N>
    /// ```
    Variable(VariableId),
    /// A fixed check term.
    ///
    /// Examples:
    /// ```ds
    /// <4>
    /// ```
    Term(TermId<StaticTerm>),
    /// A committed static value.
    ///
    /// Examples:
    /// ```ds
    /// import { N } from "./dependency"
    /// ```
    Static(dir::GlobalStaticId),
}

// assert that StaticOperand <= 64B
const _: () = assert!(std::mem::size_of::<StaticOperand>() <= 64);

impl TypeOperand {
    /// Return the variable identity when this operand has one.
    pub(in crate::check) fn variable(self) -> Option<VariableId> {
        match self {
            Self::Variable(variable) => Some(variable),
            Self::Term(_) | Self::Type(_) => None,
        }
    }

    /// Return variables referenced by this operand.
    pub(in crate::check) fn referenced_variables(
        self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        match self {
            Self::Variable(variable) => smallvec::smallvec![variable],
            Self::Term(term) => state.inference.term(term).referenced_variables(state),
            Self::Type(_) => SmallVec::new(),
        }
    }
}

impl StaticOperand {
    /// Return the variable identity when this operand has one.
    pub(in crate::check) fn variable(self) -> Option<VariableId> {
        match self {
            Self::Variable(variable) => Some(variable),
            Self::Term(_) | Self::Static(_) => None,
        }
    }

    /// Return variables referenced by this operand.
    pub(in crate::check) fn referenced_variables(
        self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        match self {
            Self::Variable(variable) => smallvec::smallvec![variable],
            Self::Term(term) => state.inference.term(term).referenced_variables(state),
            Self::Static(_) => SmallVec::new(),
        }
    }
}

impl Clone for TypeOperand {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for TypeOperand {}

impl Clone for StaticOperand {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for StaticOperand {}

impl PartialEq for TypeOperand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Variable(left), Self::Variable(right)) => left == right,
            (Self::Term(left), Self::Term(right)) => left.id == right.id,
            (Self::Type(left), Self::Type(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for TypeOperand {}

impl PartialEq for StaticOperand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Variable(left), Self::Variable(right)) => left == right,
            (Self::Term(left), Self::Term(right)) => left.id == right.id,
            (Self::Static(left), Self::Static(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for StaticOperand {}

impl Hash for TypeOperand {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Variable(variable) => {
                0_u8.hash(state);
                variable.hash(state);
            }
            Self::Term(term) => {
                1_u8.hash(state);
                term.id.hash(state);
            }
            Self::Type(ty) => {
                2_u8.hash(state);
                ty.hash(state);
            }
        }
    }
}

impl Hash for StaticOperand {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Variable(variable) => {
                0_u8.hash(state);
                variable.hash(state);
            }
            Self::Term(term) => {
                1_u8.hash(state);
                term.id.hash(state);
            }
            Self::Static(value) => {
                2_u8.hash(state);
                value.hash(state);
            }
        }
    }
}

impl Debug for TypeOperand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(variable) => f.debug_tuple("Variable").field(variable).finish(),
            Self::Term(term) => f.debug_tuple("Term").field(&term.id).finish(),
            Self::Type(ty) => f.debug_tuple("Type").field(ty).finish(),
        }
    }
}

impl Debug for StaticOperand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(variable) => f.debug_tuple("Variable").field(variable).finish(),
            Self::Term(term) => f.debug_tuple("Term").field(&term.id).finish(),
            Self::Static(value) => f.debug_tuple("Static").field(value).finish(),
        }
    }
}

impl From<VariableId> for TypeOperand {
    fn from(variable: VariableId) -> Self {
        Self::Variable(variable)
    }
}

impl From<TermId<TypeTerm>> for TypeOperand {
    fn from(term: TermId<TypeTerm>) -> Self {
        Self::Term(term)
    }
}

impl From<dir::GlobalTypeId> for TypeOperand {
    fn from(ty: dir::GlobalTypeId) -> Self {
        Self::Type(ty)
    }
}

impl From<VariableId> for StaticOperand {
    fn from(variable: VariableId) -> Self {
        Self::Variable(variable)
    }
}

impl From<TermId<StaticTerm>> for StaticOperand {
    fn from(term: TermId<StaticTerm>) -> Self {
        Self::Term(term)
    }
}

impl From<dir::GlobalStaticId> for StaticOperand {
    fn from(value: dir::GlobalStaticId) -> Self {
        Self::Static(value)
    }
}
