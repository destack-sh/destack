use tspp_repository::ProviderContext;
use tspp_source::{PackageId, TargetId};

/// State for one link phase provider run.
pub(in crate::link) struct LinkState<'a> {
    /// The package being linked.
    pub(in crate::link) package: PackageId,
    /// The target being linked.
    pub(in crate::link) target: TargetId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::link) context: &'a dyn ProviderContext,
}

impl<'a> LinkState<'a> {
    /// Create link provider state.
    pub(in crate::link) fn new(
        package: PackageId,
        target: TargetId,
        context: &'a dyn ProviderContext,
    ) -> Self {
        Self {
            package,
            target,
            context,
        }
    }
}
