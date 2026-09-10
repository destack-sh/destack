use super::Mutation;

/// Declare which mutations invalidate an analysis.
pub(crate) trait Analysis: Send + Sync + 'static {
    /// The mutations that invalidate the result or its dependencies.
    const INVALIDATED_BY: Mutation;
}
