use destack_source::ModuleId;
use destack_workspace::ProviderContext;

/// State for one declare phase provider run.
pub(in crate::declare) struct DeclareState<'a> {
    /// The module being declared.
    pub(in crate::declare) module: ModuleId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::declare) context: &'a dyn ProviderContext,
}

impl<'a> DeclareState<'a> {
    /// Create declare provider state.
    pub(in crate::declare) fn new(module: ModuleId, context: &'a dyn ProviderContext) -> Self {
        Self { module, context }
    }
}
