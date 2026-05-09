use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one check phase provider run.
pub(in crate::check) struct CheckState<'a> {
    /// The module being checked.
    pub(in crate::check) module: ModuleId,
    /// The profile being checked.
    pub(in crate::check) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::check) context: &'a dyn ProviderContext,
}

impl<'a> CheckState<'a> {
    /// Create check provider state.
    pub(in crate::check) fn new(
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
