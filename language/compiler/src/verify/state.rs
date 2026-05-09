use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderContext};

/// State for one verify phase provider run.
pub(in crate::verify) struct VerifyState<'a> {
    /// The module being verified.
    pub(in crate::verify) module: ModuleId,
    /// The profile being verified.
    pub(in crate::verify) profile: ProfileId,
    /// The target being verified.
    pub(in crate::verify) target: TargetId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::verify) context: &'a dyn ProviderContext,
}

impl<'a> VerifyState<'a> {
    /// Create verify provider state.
    pub(in crate::verify) fn new(
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &'a dyn ProviderContext,
    ) -> Self {
        Self {
            module,
            profile,
            target,
            context,
        }
    }
}
