use serde::{Deserialize, Serialize};

use destack_source::{Diagnostic, FileId, FileVersion, ModuleId, PackageId, ProfileId};

use super::FileSnapshot;

/// Record of an invalidation step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidationSummary {
    /// The file id that triggered invalidation.
    pub file_id: FileId,
    /// The updated file version.
    pub file_version: FileVersion,
    /// Invalidation kinds.
    pub kinds: Vec<InvalidationKind>,
    /// Modules invalidated by the change.
    pub modules: Vec<ModuleId>,
    /// Packages invalidated by the change.
    pub packages: Vec<PackageId>,
    /// Profiles invalidated by the change.
    pub profiles: Vec<ProfileId>,
    /// Module graph drops caused by invalidation.
    pub graphs_dropped: Vec<ProfileId>,
}

/// Invalidation kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationKind {
    /// Module source invalidation.
    ModuleSource,
    /// Dsconfig invalidation.
    DsConfig,
    /// Tsconfig invalidation.
    TsConfig,
    /// Package manifest invalidation.
    PackageManifest,
    /// Unknown invalidation.
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
    /// Invalidation summary.
    pub invalidation: InvalidationSummary,
    /// Diagnostics produced by the update.
    pub diagnostics: Vec<Diagnostic>,
}
