use std::mem::size_of;

use tspp_dir as dir;

use crate::sema::{BoundList, OriginId};

/// What one inference variable ranges over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum VariableKind {
    /// Any type.
    Type,
    /// An integer type a widened integer literal takes.
    Integer,
    /// A float type a widened float literal takes.
    Float,
    /// A memory component of one kind.
    Memory(dir::MemoryParameter),
}

impl VariableKind {
    /// Join the kinds of two unified variables, a numeric kind narrowing a type kind.
    pub(in crate::sema) fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Type, other) => other,
            (kind, Self::Type) => kind,
            (Self::Float, Self::Integer) | (Self::Integer, Self::Float) => Self::Float,
            (kind, other) if kind == other => kind,
            _ => Self::Type,
        }
    }

    /// Return whether the kind ranges over numeric types.
    pub(in crate::sema) fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Float)
    }

    /// Return the scalar domains the kind's candidates come from.
    pub(in crate::sema) fn domains(self) -> &'static [dir::ScalarDomain] {
        match self {
            Self::Type | Self::Memory(_) => &[],
            Self::Integer => &[dir::ScalarDomain::Integer, dir::ScalarDomain::Float],
            Self::Float => &[dir::ScalarDomain::Float],
        }
    }

    /// Return the literal domain of one numeric kind.
    pub(in crate::sema) fn literal_domain(self) -> Option<dir::ScalarDomain> {
        match self {
            Self::Type | Self::Memory(_) => None,
            Self::Integer => Some(dir::ScalarDomain::Integer),
            Self::Float => Some(dir::ScalarDomain::Float),
        }
    }

    /// Return the primitive the kind falls back to when nothing decides it.
    pub(in crate::sema) fn fallback(self) -> Option<dir::Type> {
        let primitive = match self {
            Self::Type | Self::Memory(_) => return None,
            Self::Integer => dir::PrimitiveType::Integer(dir::IntegerType::DEFAULT),
            Self::Float => dir::PrimitiveType::Float(dir::FloatType::Float64),
        };

        Some(dir::Type::Primitive(primitive))
    }
}

/// One inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct Variable {
    /// The interned origin that produced the variable.
    pub(in crate::sema) origin: OriginId,
    /// Bounds that must relate to the variable.
    pub(in crate::sema) lower: BoundList,
    /// Bounds the variable must relate to.
    pub(in crate::sema) upper: BoundList,
    /// The variable's inference state.
    pub(in crate::sema) state: VariableState,
    /// What the variable ranges over.
    pub(in crate::sema) kind: VariableKind,

    /// The declared generic parameter the variable instantiates.
    pub(in crate::sema) parameter: Option<dir::GlobalGenericParameterId>,
    /// The declared default completing the variable when nothing else does.
    pub(in crate::sema) default: Option<dir::GlobalTypeId>,
    /// The marks inference sets on the variable.
    pub(in crate::sema) flags: VariableFlags,
}

/// The marks inference sets on one variable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::sema) struct VariableFlags(u8);

impl VariableFlags {
    /// A variable without marks.
    pub(in crate::sema) const EMPTY: Self = Self(0);
    /// The variable is fixed to its current candidates.
    pub(in crate::sema) const FIXED: Self = Self(1 << 0);
    /// A rolled-back decision abandoned the variable.
    pub(in crate::sema) const DEAD: Self = Self(1 << 1);
    /// The variable joins its collected values.
    pub(in crate::sema) const JOIN: Self = Self(1 << 2);
    /// The variable received never and completes as never.
    pub(in crate::sema) const DIVERGING: Self = Self(1 << 3);
    /// The marks an alias forwards onto its root.
    pub(in crate::sema) const FORWARDED: Self = Self(Self::FIXED.0 | Self::JOIN.0);

    /// Return whether every bit of `other` is set.
    pub(in crate::sema) fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Set every bit of `other`.
    pub(in crate::sema) fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Return the bits set in both.
    pub(in crate::sema) fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

/// Inference state of one variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum VariableState {
    /// The variable is awaiting bounds.
    Open,
    /// The variable forwards to an equal earlier variable.
    Alias(dir::TypeVariableId),
    /// The variable inferred one type.
    Resolved(dir::GlobalTypeId),
    /// Inference failed and produced the compiler error type.
    Error(dir::GlobalTypeId),
}

impl VariableState {
    /// Return the completed type, when inference finished.
    pub(in crate::sema) fn ty(self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Open | Self::Alias(_) => None,
            Self::Resolved(ty) | Self::Error(ty) => Some(ty),
        }
    }

    /// Return whether the variable still awaits bounds.
    pub(in crate::sema) fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }
}

// lock the hot solver entry shape
#[cfg(target_pointer_width = "64")]
const _: () = assert!(size_of::<Variable>() <= 128);
