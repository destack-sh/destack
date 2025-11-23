use crate::ModuleId;

/// A AnalysisTable is a side table for high-level data flow. NOT THREAD-SAFE.
/// NOTE: low-level data flow happens in MIR, this is just for narrowing and type info.
#[derive(Debug, Clone)]
pub struct AnalysisTable {
    /// The module id of the flow table.
    pub module_id: ModuleId,
}

impl AnalysisTable {
    /// Create a new AnalysisTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self { module_id }
    }
}
