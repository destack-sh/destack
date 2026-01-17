use destack_source::{Diagnostic, FileId, ModuleId};
use destack_workspace::InvalidationPlan;

/// Summary of a daemon update.
#[derive(Debug, Clone)]
pub struct DaemonUpdate {
    /// The module id that was updated.
    pub module_id: Option<ModuleId>,
    /// The file id for the updated module.
    pub file_id: FileId,
    /// The invalidation summary for the update.
    pub invalidation: InvalidationPlan,
    /// Diagnostics for the updated file.
    pub diagnostics: Vec<Diagnostic>,
}
