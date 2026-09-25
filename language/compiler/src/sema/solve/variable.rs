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
    /// Whether a function body read the variable, fixing it to its candidates so far.
    pub(in crate::sema) is_fixed: bool,
    /// Whether a rolled-back nested decision left the variable behind.
    pub(in crate::sema) is_dead: bool,
    /// Whether the variable joins the values it collects with them.
    pub(in crate::sema) is_join: bool,
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
