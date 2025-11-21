use crate::ModuleId;

/// A FlowTable is a side table for high-level data flow. NOT THREAD-SAFE.
/// NOTE: low-level data flow happens in MIR, this is just for narrowing and type info.
#[derive(Debug, Clone)]
pub struct FlowTable {
    /// The module id of the flow table.
    pub module_id: ModuleId,
}

impl FlowTable {
    /// Create a new FlowTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self { module_id }
    }
}
