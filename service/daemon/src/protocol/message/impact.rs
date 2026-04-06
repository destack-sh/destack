use serde::{Deserialize, Serialize};

use destack_source::{Diagnostic, FileId, ModuleId, PackageId, ProfileId};

use super::FileSnapshot;

/// Record of an impact step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateImpactSummary {
    /// The file id that triggered impact.
    pub file_id: FileId,
    /// Impact kinds.
    pub kinds: Vec<UpdateImpactKind>,
    /// Modules invalidated by the change.
    pub modules: Vec<ModuleId>,
    /// Packages invalidated by the change.
    pub packages: Vec<PackageId>,
    /// Profiles invalidated by the change.
    pub profiles: Vec<ProfileId>,
    /// Module graph drops caused by impact.
    pub graphs_dropped: Vec<ProfileId>,
}

/// Impact kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateImpactKind {
    /// Module source impact.
    ModuleSource,
    /// Dsconfig impact.
    Destack,
    /// Tsconfig impact.
    TsConfig,
    /// Package manifest impact.
    PackageManifest,
    /// Unknown impact.
    Unknown,
}

/// Daemon update record for protocol responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonUpdateRecord {
    /// Module id for the update.
    pub module_id: Option<ModuleId>,
    /// File id for the update.
    pub file_id: FileId,
    /// File snapshot for the update.
    pub file: FileSnapshot,
    /// Impact summary.
    pub impact: UpdateImpactSummary,
    /// Diagnostics produced by the update.
    pub diagnostics: Vec<Diagnostic>,
}
