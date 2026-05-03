use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

/// State for one resolve phase provider run.
pub(in crate::resolve) struct ResolveState<'a> {
    /// The optional module being resolved.
    pub(in crate::resolve) module: Option<ModuleId>,
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
        Self {
            module: None,
            profile,
            context,
        }
    }

    /// Create module scoped resolve provider state.
    pub(in crate::resolve) fn module(
        module: ModuleId,
        profile: ProfileId,
        context: &'a dyn ProviderContext,
    ) -> Self {
        Self {
            module: Some(module),
            profile,
            context,
        }
    }
}
