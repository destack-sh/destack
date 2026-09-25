use std::fmt;
use std::sync::Arc;

use tspp_program as program;

use crate::binding::BindingTable;
use crate::diagnostic::RuntimeResult;
use crate::host::resource::ResourceRebinders;
use crate::machine::native;

/// External capabilities used while rebuilding one world image.
#[derive(Clone, Copy)]
pub struct RestoreContext<'a> {
    /// Resource rebinding table used for external host resources.
    resources: Option<&'a ResourceRebinders>,
    /// Native loader used to rebuild process-local native code.
    native: Option<&'a dyn native::Loader>,
    /// Runtime binding implementations used to rebuild executable runtimes.
    bindings: Option<&'a Arc<BindingTable>>,
}

impl<'a> RestoreContext<'a> {
    /// Create one empty restore context.
    pub const fn empty() -> Self {
        Self {
            resources: None,
            native: None,
            bindings: None,
        }
    }

    /// Return this context with resource rebinders.
    pub const fn with_resources(self, resources: &'a ResourceRebinders) -> Self {
        Self {
            resources: Some(resources),
            native: self.native,
            bindings: self.bindings,
        }
    }

    /// Return this context with one native loader.
    pub const fn with_native(self, native: &'a dyn native::Loader) -> Self {
        Self {
            resources: self.resources,
            native: Some(native),
            bindings: self.bindings,
        }
    }

    /// Return this context with runtime binding implementations.
    pub const fn with_bindings(self, bindings: &'a Arc<BindingTable>) -> Self {
        Self {
            resources: self.resources,
            native: self.native,
            bindings: Some(bindings),
        }
    }

    /// Return the resource rebinders.
    pub const fn resource_rebinders(self) -> Option<&'a ResourceRebinders> {
        self.resources
    }

    /// Return the native loader.
    pub const fn native_loader(self) -> Option<&'a dyn native::Loader> {
        self.native
    }

    /// Resolve runtime binding implementations for one program.
    pub(crate) fn binding_table(
        self,
        program: &program::Program,
    ) -> RuntimeResult<Arc<BindingTable>> {
        let bindings = self
            .bindings
            .cloned()
            .unwrap_or_else(|| Arc::new(BindingTable::new()));
        bindings.require(program)?;

        Ok(bindings)
    }
}

impl fmt::Debug for RestoreContext<'_> {
    /// Format this restore context without exposing loader internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RestoreContext")
            .field("resources", &self.resources.is_some())
            .field("native", &self.native.is_some())
            .field("bindings", &self.bindings.is_some())
            .finish()
    }
}

impl<'a> From<Option<&'a ResourceRebinders>> for RestoreContext<'a> {
    /// Create one restore context from resource rebinders.
    fn from(resources: Option<&'a ResourceRebinders>) -> Self {
        if let Some(resources) = resources {
            Self::empty().with_resources(resources)
        } else {
            Self::empty()
        }
    }
}
