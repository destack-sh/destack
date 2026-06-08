use destack_source::{ModuleId, TargetId};
use destack_repository::{ProfileId, ProviderContext};

/// State for one lower phase provider run.
pub(in crate::lower) struct LowerState<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The profile being lowered.
    pub(in crate::lower) profile: ProfileId,
    /// The target being lowered.
    pub(in crate::lower) target: TargetId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::lower) context: &'a dyn ProviderContext,
}

impl<'a> LowerState<'a> {
    /// Create lower provider state.
    pub(in crate::lower) fn new(
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
