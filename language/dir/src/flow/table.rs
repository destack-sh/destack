/// A FlowTable is a side table for high-level data flow. NOT THREAD-SAFE.
/// NOTE: low-level data flow happens in MIR, this is just for narrowing and type info.
#[derive(Debug, Clone)]
pub struct FlowTable {}

impl Default for FlowTable {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowTable {
    /// Create a new FlowTable.
    pub fn new() -> Self {
        Self {}
    }
}
