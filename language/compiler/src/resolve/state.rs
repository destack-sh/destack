use destack_workspace::{ProfileId, ProviderContext};

/// State for one resolve phase provider run.
pub(in crate::resolve) struct ResolveState<'a> {
    /// The profile being resolved.
    pub(in crate::resolve) profile: ProfileId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::resolve) context: &'a dyn ProviderContext,
}

impl<'a> ResolveState<'a> {
    /// Create profile scoped resolve provider state.
    pub(in crate::resolve) fn profile(
        profile: ProfileId,
        context: &'a dyn ProviderContext,
    ) -> Self {
        Self { profile, context }
    }

    /// Create module scoped resolve provider state.
    pub(in crate::resolve) fn module(profile: ProfileId, context: &'a dyn ProviderContext) -> Self {
        Self { profile, context }
    }
}
