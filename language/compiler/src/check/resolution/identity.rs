use destack_dir as dir;

use crate::check::{CheckState, TypeOperand};

/// Runtime identity equality failure resolved by the solver.
///
/// Examples:
/// ```ds
/// left === right
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IdentityFailure {
    /// The operand types do not have a shared identity domain.
    ///
    /// Examples:
    /// ```ds
    /// left === right
    /// ```
    Incompatible,
}

/// Runtime identity equality decision resolved by the solver.
///
/// Examples:
/// ```ds
/// left === right
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum IdentityDecision {
    /// One identity comparison resolved.
    ///
    /// Examples:
    /// ```ds
    /// left === right
    /// ```
    Resolved(IdentitySelection),
    /// Identity comparison resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// left === right
    /// ```
    Rejected(IdentityFailure),
}

/// Runtime identity comparison selected by the solver before commit.
///
/// Examples:
/// ```ds
/// left === right
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct IdentitySelection {
    /// The source identity expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The left operand type.
    pub(in crate::check) left: TypeOperand,
    /// The right operand type.
    pub(in crate::check) right: TypeOperand,
}

impl CheckState<'_> {
    /// Select one identity equality decision.
    pub(in crate::check) fn select_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentityDecision,
    ) {
        if let Some(previous) = self.inference.identity(source) {
            assert_eq!(
                previous, decision,
                "check identity {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_identity(source, decision);
    }
}
