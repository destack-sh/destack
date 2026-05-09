use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one import phase provider run.
pub(in crate::import) struct ImportState<'a> {
    /// The module being imported.
    pub(in crate::import) module: ModuleId,
    /// The profile being imported.
    pub(in crate::import) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::import) context: &'a dyn ProviderContext,
}

impl<'a> ImportState<'a> {
    /// Create module scoped import provider state.
    pub(in crate::import) fn new(
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
