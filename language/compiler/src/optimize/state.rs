use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

/// State for one optimize phase provider run.
pub(in crate::optimize) struct OptimizeState<'a> {
    /// The module being optimized.
    pub(in crate::optimize) module: ModuleId,
    /// The profile being optimized.
    pub(in crate::optimize) profile: ProfileId,
    /// The target being optimized.
    pub(in crate::optimize) target: TargetId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::optimize) context: &'a dyn ProviderContext,
}

impl<'a> OptimizeState<'a> {
    /// Create optimize provider state.
    pub(in crate::optimize) fn new(
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
