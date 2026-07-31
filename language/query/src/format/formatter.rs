use crate::{ModuleQueryContext, ProgramQueryContext};

/// Formatter for query display text.
pub(crate) struct Formatter<'owner, 'module, 'query> {
    /// The module that owns local ids read by the formatter.
    pub(super) module: &'owner ModuleQueryContext<'module>,
    /// The shared query context.
    pub(super) query: &'owner ProgramQueryContext<'query>,
}

impl<'owner, 'module, 'query> Formatter<'owner, 'module, 'query> {
    /// Create a formatter for one module.
    pub(crate) fn new(
        module: &'owner ModuleQueryContext<'module>,
        query: &'owner ProgramQueryContext<'query>,
    ) -> Self {
        Self { module, query }
    }
}
