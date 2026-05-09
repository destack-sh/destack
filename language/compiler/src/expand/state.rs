use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one expand phase provider run.
pub(in crate::expand) struct ExpandState<'a> {
    /// The module being expanded.
    pub(in crate::expand) module: ModuleId,
    /// The profile being expanded.
    pub(in crate::expand) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::expand) context: &'a dyn ProviderContext,
}

impl<'a> ExpandState<'a> {
    /// Create expand provider state.
    pub(in crate::expand) fn new(
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
