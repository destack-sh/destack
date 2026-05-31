use destack_dir as dir;

use crate::check::{CandidateSelection, CheckState, GenericApplication, TypeOperand};

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
    Missing,
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
    Resolved(MemberSelection),
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
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberSelection {
    /// The source member expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver type.
    pub(in crate::check) receiver: TypeOperand,
    /// The resolved member target.
    pub(in crate::check) target: MemberTargetSelection,
}

/// Solved member target resolved by the solver.
///
/// Examples:
/// ```ds
/// value.member
/// value[index]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberTargetSelection {
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
        /// The resolved generic application.
        application: Option<GenericApplication>,
    },
    /// Symbol-backed members selected from a union receiver.
    ///
    /// Examples:
    /// ```ds
    /// value.method
    /// ```
    Union(Vec<CandidateSelection>),
}

impl CheckState<'_> {
    /// Select one member decision.
    pub(in crate::check) fn select_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberDecision,
    ) {
        if let Some(previous) = self.inference.member(source) {
            assert_eq!(
                previous, decision,
                "check member {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_member(source, decision);
    }
}
