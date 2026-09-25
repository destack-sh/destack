use tspp_core::FxIndexMap;
use tspp_dir as dir;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryResult};

/// The reopening one declared membership formats under.
#[derive(Debug, Clone, Default)]
pub(crate) struct Reopening {
    /// The use-site argument per declared parameter.
    pub(crate) parameters: FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>,
    /// The use-site receiver `this` reopens at.
    pub(crate) receiver: Option<dir::GlobalTypeId>,
}

/// Formatter for query display text.
pub(crate) struct Formatter<'owner, 'module, 'program> {
    /// The module that owns local ids read by the formatter.
    pub(super) module: &'owner ModuleQueryContext<'module>,
    /// The queried program.
    pub(super) program: &'owner ProgramQueryContext<'program>,
    /// The reopening declared member types format under.
    pub(super) reopening: Option<&'owner Reopening>,
}

impl<'owner, 'module, 'program> Formatter<'owner, 'module, 'program> {
    /// Create a formatter for one module.
    pub(crate) fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
    ) -> Self {
        Self {
            module,
            program,
            reopening: None,
        }
    }

    /// Format declared member types under one reopening, when any.
    pub(crate) fn with_reopening(mut self, reopening: Option<&'owner Reopening>) -> Self {
        self.reopening = reopening;

        self
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
            self.program.read_type(type_id, |type_value, module| {
                // carry the reopening into the owning module's formatter
                let mut formatter = Formatter::new(module, self.program);
                formatter.reopening = self.reopening;

                read(type_value, &formatter)
            })
        }
    }
}
