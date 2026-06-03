use destack_dir as dir;

use crate::CompilerResult;
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
    Resolved(IdentityResolution),
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
pub(in crate::check) struct IdentityResolution {
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
    ) -> CompilerResult<()> {
        if let Some(existing) = self.inference.identity(source) {
            if existing == decision {
                return Ok(());
            }

            return Err(self.selection_conflict_error("identity", source, &existing, &decision));
        }

        self.inference.select_identity(source, decision);

        Ok(())
    }
}
