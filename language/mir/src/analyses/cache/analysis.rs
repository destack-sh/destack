use crate as mir;

use super::Mutation;

/// Declare which mutations invalidate an analysis.
pub(crate) trait Analysis: Send + Sync + 'static {
    /// The mutations that invalidate the result or its dependencies.
    const INVALIDATED_BY: Mutation;
}

/// Inputs shared by MIR analyses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AnalysisOptions {
    /// Target layout used by layout sensitive analyses.
    pub target_layout: mir::TargetLayout,
}

impl AnalysisOptions {
    /// Create analysis options for one target.
    pub fn new(target_layout: mir::TargetLayout) -> Self {
        Self { target_layout }
    }
}
