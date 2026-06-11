use destack_dir as dir;

use crate::check::{CandidateResolution, GenericInstance, TypeOperand};

/// Runtime member failure resolved by the solver.
///
/// Examples:
/// ```ds
/// value.missing
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum MemberFailure {
    /// The receiver has no such member.
    ///
    /// Examples:
    /// ```ds
    /// value.missing
    /// ```
    Missing {
        /// The selected member key.
        key: dir::StaticKey,
    },
}

/// Runtime member decision resolved by the solver.
///
/// Examples:
/// ```ds
/// value.member
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberDecision {
    /// One member target resolved.
    ///
    /// Examples:
    /// ```ds
    /// value.member
    /// ```
    Resolved(MemberResolution),
    /// Member resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// value.missing
    /// ```
    Rejected(MemberFailure),
}

/// Runtime member selected by the solver before commit.
///
/// Examples:
/// ```ds
/// value.member
/// value["member"]
/// ```
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberResolution {
    /// The source member expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver type.
    pub(in crate::check) receiver: TypeOperand,
    /// The resolved member target.
    pub(in crate::check) target: MemberTargetResolution,
}

impl PartialEq for MemberResolution {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.target == other.target
    }
}

/// Solved member target resolved by the solver.
///
/// Examples:
/// ```ds
/// value.member
/// value[index]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberTargetResolution {
    /// Compiler builtin member behavior.
    ///
    /// Examples:
    /// ```ds
    /// value[index]
    /// value[start..end]
    /// ```
    Builtin(dir::BuiltinMember),
    /// Structural field resolved from a shape type.
    ///
    /// Examples:
    /// ```ds
    /// value.field
    /// ```
    Field(dir::StaticKey),
    /// Symbol-backed member resolved from a nominal type.
    ///
    /// Examples:
    /// ```ds
    /// value.method
    /// ```
    Symbol {
        /// The resolved member symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
    },
    /// Symbol-backed members selected from a union receiver.
    ///
    /// Examples:
    /// ```ds
    /// value.method
    /// ```
    Union(Vec<CandidateResolution>),
}
