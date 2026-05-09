use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one materialize phase provider run.
pub(in crate::materialize) struct MaterializeState<'a> {
    /// The module being materialized.
    pub(in crate::materialize) module: ModuleId,
    /// The profile being materialized.
    pub(in crate::materialize) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::materialize) context: &'a dyn ProviderContext,
}

impl<'a> MaterializeState<'a> {
    /// Create materialize provider state.
    pub(in crate::materialize) fn new(
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
