use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one export phase provider run.
pub(in crate::export) struct ExportState<'a> {
    /// The module being exported.
    pub(in crate::export) module: ModuleId,
    /// The profile being exported.
    pub(in crate::export) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::export) context: &'a dyn ProviderContext,
}

impl<'a> ExportState<'a> {
    /// Create export provider state.
    pub(in crate::export) fn new(
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
