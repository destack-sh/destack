use crate::{ModuleId, bridge};

/// One module loaded through a live session.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Stable source module id.
    pub id: ModuleId,
}

impl Module {
    /// Create one module value.
    pub fn new(id: ModuleId) -> Self {
        Self { id }
    }
}
