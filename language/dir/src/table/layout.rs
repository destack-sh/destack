use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

/// Checked layouts for one DIR module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutTable {
    /// The module id of the layout table.
    pub module_id: ModuleId,
}

impl LayoutTable {
    /// Create an empty layout table.
    pub fn new(module_id: ModuleId) -> Self {
        Self { module_id }
    }
}
