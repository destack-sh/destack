use serde::{Deserialize, Serialize};

/// The semantic relation between two source modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ModuleRelation {
    /// Binding import.
    Import,
    /// Binding re-export.
    ReExport,
    /// Non-binding module reference.
    Reference,
}
