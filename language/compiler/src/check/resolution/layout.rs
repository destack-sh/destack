use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Layout, LayoutQuery, TypeOperand};

/// Layout query failure resolved by the solver.
///
/// Examples:
/// ```ds
/// sizeOf<T>()
/// alignOf<T>()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct LayoutFailure {
    /// The source layout query expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The queried type.
    pub(in crate::check) target: TypeOperand,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
}

/// Layout query decision resolved by the solver.
///
/// Examples:
/// ```ds
/// sizeOf<T>()
/// alignOf<T>()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum LayoutDecision {
    /// One layout resolved.
    ///
    /// Examples:
    /// ```ds
    /// sizeOf<int32>()
    /// ```
    Resolved(LayoutResolution),
    /// Layout resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// sizeOf<T>()
    /// ```
    Rejected(LayoutFailure),
}

/// Layout selected by the solver before commit.
///
/// Examples:
/// ```ds
/// sizeOf<int32>()
/// alignOf<int32>()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct LayoutResolution {
    /// The source layout query expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The queried type.
    pub(in crate::check) target: TypeOperand,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
    /// The resolved layout.
    pub(in crate::check) layout: Layout,
}

impl CheckState<'_> {
    /// Select one concrete layout decision.
    pub(in crate::check) fn select_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutDecision,
    ) -> CompilerResult<()> {
        if let Some(existing) = self.inference.layout(source) {
            if existing == decision {
                return Ok(());
            }

            return Err(self.selection_conflict_error("layout", source, &existing, &decision));
        }

        self.inference.select_layout(source, decision);

        Ok(())
    }
}
