use destack_dir as dir;

use crate::check::{Layout, LayoutQuery, TypeOperand};

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
