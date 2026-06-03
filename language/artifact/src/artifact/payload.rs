use crate::{
    Data, DependencyIndex, DirBound, DirChecked, DirCheckedComponent, DirElaborated, DirExpanded,
    DirExported, DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment,
    MirLowered, MirOptimized, MirVerified, ModuleLinted, ModuleOutput, ModuleQueryIndex,
    PackageLinted, PackageOutput, WorkspaceLinted, WorkspaceQueryIndex,
};
use serde::{Deserialize, Serialize};

/// One typed artifact payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactPayload {
    /// Parsed module DIR.
    DirParsed(DirParsed),
    /// Parsed non-code module data.
    Data(Data),
    /// Explicit global environment for one profile.
    GlobalEnvironment(GlobalEnvironment),
    /// Active dependency index for one profile.
    DependencyIndex(DependencyIndex),
    /// Bound DIR.
    DirBound(DirBound),
    /// Imported DIR.
    DirImported(DirImported),
    /// Expanded DIR.
    DirExpanded(DirExpanded),
    /// Exported DIR.
    DirExported(DirExported),
    /// Resolved DIR imports.
    DirResolved(DirResolved),
    /// Checked DIR component.
    DirCheckedComponent(DirCheckedComponent),
    /// Checked DIR facade.
    DirChecked(DirChecked),
    /// Materialized DIR.
    DirMaterialized(DirMaterialized),
    /// Elaborated DIR.
    DirElaborated(DirElaborated),
    /// Lowered MIR before optimization.
    MirLowered(MirLowered),
    /// Verified MIR marker after required semantic verification.
    MirVerified(MirVerified),
    /// Optimized MIR.
    MirOptimized(MirOptimized),
    /// Query index for one module profile.
    ModuleQueryIndex(ModuleQueryIndex),
    /// Query index for one workspace profile.
    WorkspaceQueryIndex(WorkspaceQueryIndex),
    /// One generated module output for one target.
    ModuleOutput(ModuleOutput),
    /// Output entries for one package target.
    PackageOutput(PackageOutput),
    /// Realized lint diagnostics for one module profile.
    ModuleLinted(ModuleLinted),
    /// Realized lint diagnostics for one package.
    PackageLinted(PackageLinted),
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted(WorkspaceLinted),
}

/// Borrowed artifact payload used for transport serialization.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum ArtifactPayloadRef<'a> {
    /// Parsed module DIR.
    DirParsed(&'a DirParsed),
    /// Parsed non-code module data.
    Data(&'a Data),
    /// Explicit global environment for one profile.
    GlobalEnvironment(&'a GlobalEnvironment),
    /// Active dependency index for one profile.
    DependencyIndex(&'a DependencyIndex),
    /// Bound DIR.
    DirBound(&'a DirBound),
    /// Imported DIR.
    DirImported(&'a DirImported),
    /// Expanded DIR.
    DirExpanded(&'a DirExpanded),
    /// Exported DIR.
    DirExported(&'a DirExported),
    /// Resolved DIR imports.
    DirResolved(&'a DirResolved),
    /// Checked DIR component.
    DirCheckedComponent(&'a DirCheckedComponent),
    /// Checked DIR facade.
    DirChecked(&'a DirChecked),
    /// Materialized DIR.
    DirMaterialized(&'a DirMaterialized),
    /// Elaborated DIR.
    DirElaborated(&'a DirElaborated),
    /// Lowered MIR before optimization.
    MirLowered(&'a MirLowered),
    /// Verified MIR marker after required semantic verification.
    MirVerified(&'a MirVerified),
    /// Optimized MIR.
    MirOptimized(&'a MirOptimized),
    /// Query index for one module profile.
    ModuleQueryIndex(&'a ModuleQueryIndex),
    /// Query index for one workspace profile.
    WorkspaceQueryIndex(&'a WorkspaceQueryIndex),
    /// One generated module output for one target.
    ModuleOutput(&'a ModuleOutput),
    /// Output entries for one package target.
    PackageOutput(&'a PackageOutput),
    /// Realized lint diagnostics for one module profile.
    ModuleLinted(&'a ModuleLinted),
    /// Realized lint diagnostics for one package.
    PackageLinted(&'a PackageLinted),
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted(&'a WorkspaceLinted),
}

impl ArtifactPayload {
    /// Return the stable short name for this payload kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment(_) => "global_environment",
            Self::DependencyIndex(_) => "dependency_index",
            Self::DirParsed(_) => "dir_parsed",
            Self::Data(_) => "data",
            Self::DirBound(_) => "dir_bound",
            Self::DirImported(_) => "dir_imported",
            Self::DirExpanded(_) => "dir_expanded",
            Self::DirExported(_) => "dir_exported",
            Self::DirResolved(_) => "dir_resolved",
            Self::DirCheckedComponent(_) => "dir_checked_component",
            Self::DirChecked(_) => "dir_checked",
            Self::DirMaterialized(_) => "dir_materialized",
            Self::DirElaborated(_) => "dir_elaborated",
            Self::MirLowered(_) => "mir_lowered",
            Self::MirVerified(_) => "mir_verified",
            Self::MirOptimized(_) => "mir_optimized",
            Self::ModuleQueryIndex(_) => "module_query_index",
            Self::WorkspaceQueryIndex(_) => "workspace_query_index",
            Self::ModuleOutput(_) => "module_output",
            Self::PackageOutput(_) => "package_output",
            Self::ModuleLinted(_) => "module_linted",
            Self::PackageLinted(_) => "package_linted",
            Self::WorkspaceLinted(_) => "workspace_linted",
        }
    }
}

impl From<DirParsed> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirParsed) -> Self {
        Self::DirParsed(payload)
    }
}

impl From<Data> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Data) -> Self {
        Self::Data(payload)
    }
}

impl From<GlobalEnvironment> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: GlobalEnvironment) -> Self {
        Self::GlobalEnvironment(payload)
    }
}

impl From<DependencyIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DependencyIndex) -> Self {
        Self::DependencyIndex(payload)
    }
}

impl From<DirBound> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirBound) -> Self {
        Self::DirBound(payload)
    }
}

impl From<DirImported> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirImported) -> Self {
        Self::DirImported(payload)
    }
}

impl From<DirExpanded> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirExpanded) -> Self {
        Self::DirExpanded(payload)
    }
}

impl From<DirExported> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirExported) -> Self {
        Self::DirExported(payload)
    }
}

impl From<DirResolved> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirResolved) -> Self {
        Self::DirResolved(payload)
    }
}

impl From<DirCheckedComponent> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirCheckedComponent) -> Self {
        Self::DirCheckedComponent(payload)
    }
}

impl From<DirChecked> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirChecked) -> Self {
        Self::DirChecked(payload)
    }
}

impl From<DirMaterialized> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirMaterialized) -> Self {
        Self::DirMaterialized(payload)
    }
}

impl From<DirElaborated> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirElaborated) -> Self {
        Self::DirElaborated(payload)
    }
}

impl From<MirLowered> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirLowered) -> Self {
        Self::MirLowered(payload)
    }
}

impl From<MirVerified> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirVerified) -> Self {
        Self::MirVerified(payload)
    }
}

impl From<MirOptimized> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirOptimized) -> Self {
        Self::MirOptimized(payload)
    }
}

impl From<ModuleQueryIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleQueryIndex) -> Self {
        Self::ModuleQueryIndex(payload)
    }
}

impl From<WorkspaceQueryIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: WorkspaceQueryIndex) -> Self {
        Self::WorkspaceQueryIndex(payload)
    }
}

impl From<ModuleOutput> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleOutput) -> Self {
        Self::ModuleOutput(payload)
    }
}

impl From<PackageOutput> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageOutput) -> Self {
        Self::PackageOutput(payload)
    }
}

impl From<ModuleLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleLinted) -> Self {
        Self::ModuleLinted(payload)
    }
}

impl From<PackageLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageLinted) -> Self {
        Self::PackageLinted(payload)
    }
}

impl From<WorkspaceLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: WorkspaceLinted) -> Self {
        Self::WorkspaceLinted(payload)
    }
}
