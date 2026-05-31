use destack_dir as dir;

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
    Resolved(LayoutSelection),
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
pub(in crate::check) struct LayoutSelection {
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
    ) {
        if let Some(previous) = self.inference.layout(source) {
            assert_eq!(
                previous, decision,
                "check layout {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_layout(source, decision);
    }
}
