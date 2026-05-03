use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one analyze phase provider run.
pub(in crate::analyze) struct AnalyzeState<'a> {
    /// The module being analyzed.
    pub(in crate::analyze) module: ModuleId,
    /// The profile being analyzed.
    pub(in crate::analyze) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::analyze) context: &'a dyn ProviderContext,
}

impl<'a> AnalyzeState<'a> {
    /// Create analyze provider state.
    pub(in crate::analyze) fn new(
        module: ModuleId,
        profile: ProfileId,
        context: &'a dyn ProviderContext,
    ) -> Self {
        Self {
            module,
            profile,
            context,
        }
    }
}
