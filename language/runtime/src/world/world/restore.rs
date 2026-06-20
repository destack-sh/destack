use destack_native as native;
use std::fmt;

use crate::host::resource::ResourceRebinders;

/// External capabilities used while rebuilding one world image.
#[derive(Clone, Copy)]
pub struct RestoreContext<'a> {
    /// Resource rebinding table used for external host resources.
    resources: Option<&'a ResourceRebinders>,
    /// Native linker used to rebuild process-local native code.
    native: Option<&'a dyn native::Linker>,
}

impl<'a> RestoreContext<'a> {
    /// Create one empty restore context.
    pub const fn empty() -> Self {
        Self {
            resources: None,
            native: None,
        }
    }

    /// Return this context with resource rebinders.
    pub const fn with_resources(self, resources: &'a ResourceRebinders) -> Self {
        Self {
            resources: Some(resources),
            native: self.native,
        }
    }

    /// Return this context with one native linker.
    pub const fn with_native(self, native: &'a dyn native::Linker) -> Self {
        Self {
            resources: self.resources,
            native: Some(native),
        }
    }

    /// Return the resource rebinders.
    pub const fn resource_rebinders(self) -> Option<&'a ResourceRebinders> {
        self.resources
    }

    /// Return the native linker.
    pub const fn native_linker(self) -> Option<&'a dyn native::Linker> {
        self.native
    }
}

impl fmt::Debug for RestoreContext<'_> {
    /// Format this restore context without exposing linker internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RestoreContext")
            .field("resources", &self.resources.is_some())
            .field("native", &self.native.is_some())
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
