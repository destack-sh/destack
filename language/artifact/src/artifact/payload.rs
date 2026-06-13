use std::sync::Arc;

use crate::{
    ComponentGraph, Data, DirBound, DirChecked, DirCheckedComponent, DirElaborated, DirExpanded,
    DirExported, DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment,
    MirLowered, MirOptimized, MirVerified, ModuleIndex, ModuleLinted, ModuleOutput,
    ModuleQueryIndex, PackageIndex, PackageLinted, PackageOutput, WorkspaceLinted,
    WorkspaceQueryIndex,
};
use serde::{Deserialize, Serialize};

/// One typed artifact payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactPayload {
    /// Parsed module DIR.
    DirParsed(Arc<DirParsed>),
    /// Parsed non-code module data.
    Data(Arc<Data>),
    /// Explicit global environment for one profile.
    GlobalEnvironment(Arc<GlobalEnvironment>),
    /// Active dependency index for one profile.
    PackageIndex(Arc<PackageIndex>),
    /// Module import edges for one profile.
    ModuleIndex(Arc<ModuleIndex>),
    /// Component partition for one profile.
    ComponentGraph(Arc<ComponentGraph>),
    /// Bound DIR.
    DirBound(Arc<DirBound>),
    /// Imported DIR.
    DirImported(Arc<DirImported>),
    /// Expanded DIR.
    DirExpanded(Arc<DirExpanded>),
    /// Exported DIR.
    DirExported(Arc<DirExported>),
    /// Resolved DIR imports.
    DirResolved(Arc<DirResolved>),
    /// Checked DIR component.
    DirCheckedComponent(Arc<DirCheckedComponent>),
    /// Checked DIR facade.
    DirChecked(Arc<DirChecked>),
    /// Materialized DIR.
    DirMaterialized(Arc<DirMaterialized>),
    /// Elaborated DIR.
    DirElaborated(Arc<DirElaborated>),
    /// Lowered MIR before optimization.
    MirLowered(Arc<MirLowered>),
    /// Verified MIR marker after required semantic verification.
    MirVerified(Arc<MirVerified>),
    /// Optimized MIR.
    MirOptimized(Arc<MirOptimized>),
    /// Query index for one module profile.
    ModuleQueryIndex(Arc<ModuleQueryIndex>),
    /// Query index for one workspace profile.
    WorkspaceQueryIndex(Arc<WorkspaceQueryIndex>),
    /// One generated module output for one target.
    ModuleOutput(Arc<ModuleOutput>),
    /// Output entries for one package target.
    PackageOutput(Arc<PackageOutput>),
    /// Realized lint diagnostics for one module profile.
    ModuleLinted(Arc<ModuleLinted>),
    /// Realized lint diagnostics for one package.
    PackageLinted(Arc<PackageLinted>),
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted(Arc<WorkspaceLinted>),
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
    PackageIndex(&'a PackageIndex),
    /// Module import edges for one profile.
    ModuleIndex(&'a ModuleIndex),
    /// Component partition for one profile.
    ComponentGraph(&'a ComponentGraph),
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
            Self::PackageIndex(_) => "package_index",
            Self::ModuleIndex(_) => "module_index",
            Self::ComponentGraph(_) => "component_graph",
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
        Self::DirParsed(Arc::new(payload))
    }
}

impl From<Data> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Data) -> Self {
        Self::Data(Arc::new(payload))
    }
}

impl From<GlobalEnvironment> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: GlobalEnvironment) -> Self {
        Self::GlobalEnvironment(Arc::new(payload))
    }
}

impl From<PackageIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageIndex) -> Self {
        Self::PackageIndex(Arc::new(payload))
    }
}

impl From<ModuleIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleIndex) -> Self {
        Self::ModuleIndex(Arc::new(payload))
    }
}

impl From<ComponentGraph> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ComponentGraph) -> Self {
        Self::ComponentGraph(Arc::new(payload))
    }
}

impl From<DirBound> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirBound) -> Self {
        Self::DirBound(Arc::new(payload))
    }
}

impl From<DirImported> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirImported) -> Self {
        Self::DirImported(Arc::new(payload))
    }
}

impl From<DirExpanded> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirExpanded) -> Self {
        Self::DirExpanded(Arc::new(payload))
    }
}

impl From<DirExported> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirExported) -> Self {
        Self::DirExported(Arc::new(payload))
    }
}

impl From<DirResolved> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirResolved) -> Self {
        Self::DirResolved(Arc::new(payload))
    }
}

impl From<DirCheckedComponent> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirCheckedComponent) -> Self {
        Self::DirCheckedComponent(Arc::new(payload))
    }
}

impl From<DirChecked> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirChecked) -> Self {
        Self::DirChecked(Arc::new(payload))
    }
}

impl From<DirMaterialized> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirMaterialized) -> Self {
        Self::DirMaterialized(Arc::new(payload))
    }
}

impl From<DirElaborated> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirElaborated) -> Self {
        Self::DirElaborated(Arc::new(payload))
    }
}

impl From<MirLowered> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirLowered) -> Self {
        Self::MirLowered(Arc::new(payload))
    }
}

impl From<MirVerified> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirVerified) -> Self {
        Self::MirVerified(Arc::new(payload))
    }
}

impl From<MirOptimized> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirOptimized) -> Self {
        Self::MirOptimized(Arc::new(payload))
    }
}

impl From<ModuleQueryIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleQueryIndex) -> Self {
        Self::ModuleQueryIndex(Arc::new(payload))
    }
}

impl From<WorkspaceQueryIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: WorkspaceQueryIndex) -> Self {
        Self::WorkspaceQueryIndex(Arc::new(payload))
    }
}

impl From<ModuleOutput> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleOutput) -> Self {
        Self::ModuleOutput(Arc::new(payload))
    }
}

impl From<PackageOutput> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageOutput) -> Self {
        Self::PackageOutput(Arc::new(payload))
    }
}

impl From<ModuleLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleLinted) -> Self {
        Self::ModuleLinted(Arc::new(payload))
    }
}

impl From<PackageLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageLinted) -> Self {
        Self::PackageLinted(Arc::new(payload))
    }
}

impl From<WorkspaceLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: WorkspaceLinted) -> Self {
        Self::WorkspaceLinted(Arc::new(payload))
    }
}
