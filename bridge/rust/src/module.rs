use destack_bridge_language as bridge;
use destack_source as source;

use crate::{Error, Result};

/// One module loaded through a Rust bridge workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Module {
    /// Stable source module id.
    pub id: source::ModuleId,
}

impl Module {
    /// Create one module value.
    pub fn new(id: source::ModuleId) -> Self {
        Self { id }
    }

    /// Convert this Rust module into one bridge module.
    pub fn into_bridge(self) -> bridge::Module {
        bridge::Module::new(self.id.into())
    }
}

impl TryFrom<bridge::Module> for Module {
    type Error = Error;

    /// Convert one bridge module into one Rust module.
    fn try_from(module: bridge::Module) -> Result<Self> {
        let id = module.id.into_source().map_err(Error::new)?;

        Ok(Self { id })
    }
}

impl From<source::ModuleId> for Module {
    /// Convert one source module id into one Rust module.
    fn from(id: source::ModuleId) -> Self {
        Self::new(id)
    }
}

impl From<Module> for bridge::Module {
    /// Convert one Rust module into one bridge module.
    fn from(module: Module) -> Self {
        module.into_bridge()
    }
}
