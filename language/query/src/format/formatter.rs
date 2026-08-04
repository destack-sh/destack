use destack_dir as dir;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryResult};

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

    /// Return the type table visible to this formatter.
    pub(super) fn types(&self) -> QueryResult<&dir::TypeTable<'_>> {
        self.module.types()
    }

    /// Read one type through the table that owns its local id.
    pub(super) fn read_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &Formatter<'_, '_, '_>) -> QueryResult<R>,
    ) -> QueryResult<R> {
        if type_id.module_id == self.module.module_id() {
            let type_value = self.types()?.get_type(type_id.local_id);

            read(&type_value, self)
        } else {
            self.query.read_type(type_id, |type_value, module| {
                let formatter = Formatter::new(module, self.query);

                read(type_value, &formatter)
            })
        }
    }
}
