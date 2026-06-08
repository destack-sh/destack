use crate::bridge;

/// One module loaded through a live session.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Displayed compiler module id.
    pub module_id: String,
}

impl Module {
    /// Create one module value.
    pub fn new(module_id: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
        }
    }
}
