use std::sync::Arc;

use destack_program::Program;
use destack_serde::Reflect;
use destack_source::ContentId;

use crate::{
    ArtifactKey, ArtifactProjectionFingerprint, ArtifactProjectionKey, Asset, Build, Bundle,
    ComponentGraph, Data, DirBound, DirChecked, DirCheckedComponent, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, GlobalEnvironment, MirAnalyzed,
    MirElaborated, MirLowered, MirOptimized, MirVerified, ModuleIndex, ModuleLinted, Object,
    PackageIndex, PackageLinted, Product, ProgramAnalysis, ProgramIndex, Script, WorkspaceLinted,
};
use serde::{Deserialize, Serialize};

/// One typed artifact payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum ArtifactPayload {
    /// Parsed module DIR.
    DirParsed(Arc<DirParsed>),
    /// Parsed non-code module data.
    Data(Arc<Data>),
    /// Explicit global environment for one profile.
    GlobalEnvironment(Arc<GlobalEnvironment>),
    /// Active dependency index for one profile.
    PackageIndex(Arc<PackageIndex>),
    /// Component partition for one profile.
    ComponentGraph(Arc<ComponentGraph>),
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis(Arc<ProgramAnalysis>),
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
    /// Lowered MIR before optimization.
    MirLowered(Arc<MirLowered>),
    /// Verified MIR marker after required semantic verification.
    MirVerified(Arc<MirVerified>),
    /// MIR after required executable elaboration.
    MirElaborated(Arc<MirElaborated>),
    /// Per-module link summary for whole-program analysis.
    MirAnalyzed(Arc<MirAnalyzed>),
    /// Optimized MIR.
    MirOptimized(Arc<MirOptimized>),
    /// Index for one module profile.
    ModuleIndex(Arc<ModuleIndex>),
    /// Index for one program profile.
    ProgramIndex(Arc<ProgramIndex>),
    /// One structured linker input for one target.
    Script(Arc<Script>),
    /// One compiled-code linker input for one target.
    Object(Arc<Object>),
    /// One opaque linker input for one target.
    Asset(Arc<Asset>),
    /// Target-built toolchain payload.
    Build(Arc<Build>),
    /// Linked file graph for one package target.
    Bundle(Arc<Bundle>),
    /// Program for one package target.
    Program(Arc<Program>),
    /// Linked product assembled from configured target artifacts.
    Product(Arc<Product>),
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
    /// Component partition for one profile.
    ComponentGraph(&'a ComponentGraph),
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis(&'a ProgramAnalysis),
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
    /// Lowered MIR before optimization.
    MirLowered(&'a MirLowered),
    /// Verified MIR marker after required semantic verification.
    MirVerified(&'a MirVerified),
    /// MIR after required executable elaboration.
    MirElaborated(&'a MirElaborated),
    /// Per-module link summary for whole-program analysis.
    MirAnalyzed(&'a MirAnalyzed),
    /// Optimized MIR.
    MirOptimized(&'a MirOptimized),
    /// Index for one module profile.
    ModuleIndex(&'a ModuleIndex),
    /// Index for one program profile.
    ProgramIndex(&'a ProgramIndex),
    /// One structured linker input for one target.
    Script(&'a Script),
    /// One compiled-code linker input for one target.
    Object(&'a Object),
    /// One opaque linker input for one target.
    Asset(&'a Asset),
    /// Target-built toolchain payload.
    Build(&'a Build),
    /// Linked file graph for one package target.
    Bundle(&'a Bundle),
    /// Program for one package target.
    Program(&'a Program),
    /// Linked product assembled from configured target artifacts.
    Product(&'a Product),
    /// Realized lint diagnostics for one module profile.
    ModuleLinted(&'a ModuleLinted),
    /// Realized lint diagnostics for one package.
    PackageLinted(&'a PackageLinted),
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted(&'a WorkspaceLinted),
}

impl ArtifactPayload {
    /// Return whether this payload belongs to one artifact key.
    pub fn matches_key(&self, key: &ArtifactKey) -> bool {
        matches!(
            (key, self),
            (
                ArtifactKey::GlobalEnvironment { .. },
                ArtifactPayload::GlobalEnvironment(_)
            ) | (
                ArtifactKey::PackageIndex { .. },
                ArtifactPayload::PackageIndex(_)
            ) | (
                ArtifactKey::ComponentGraph { .. },
                ArtifactPayload::ComponentGraph(_)
            ) | (
                ArtifactKey::ProgramAnalysis { .. },
                ArtifactPayload::ProgramAnalysis(_)
            ) | (ArtifactKey::DirParsed { .. }, ArtifactPayload::DirParsed(_))
                | (ArtifactKey::Data { .. }, ArtifactPayload::Data(_))
                | (ArtifactKey::DirBound { .. }, ArtifactPayload::DirBound(_))
                | (
                    ArtifactKey::DirImported { .. },
                    ArtifactPayload::DirImported(_)
                )
                | (
                    ArtifactKey::DirExpanded { .. },
                    ArtifactPayload::DirExpanded(_)
                )
                | (
                    ArtifactKey::DirExported { .. },
                    ArtifactPayload::DirExported(_)
                )
                | (
                    ArtifactKey::DirResolved { .. },
                    ArtifactPayload::DirResolved(_)
                )
                | (
                    ArtifactKey::DirCheckedComponent { .. },
                    ArtifactPayload::DirCheckedComponent(_)
                )
                | (
                    ArtifactKey::DirChecked { .. },
                    ArtifactPayload::DirChecked(_)
                )
                | (
                    ArtifactKey::DirMaterialized { .. },
                    ArtifactPayload::DirMaterialized(_)
                )
                | (
                    ArtifactKey::MirLowered { .. },
                    ArtifactPayload::MirLowered(_)
                )
                | (
                    ArtifactKey::MirVerified { .. },
                    ArtifactPayload::MirVerified(_)
                )
                | (
                    ArtifactKey::MirElaborated { .. },
                    ArtifactPayload::MirElaborated(_)
                )
                | (
                    ArtifactKey::MirAnalyzed { .. },
                    ArtifactPayload::MirAnalyzed(_)
                )
                | (
                    ArtifactKey::MirOptimized { .. },
                    ArtifactPayload::MirOptimized(_)
                )
                | (
                    ArtifactKey::ModuleIndex { .. },
                    ArtifactPayload::ModuleIndex(_)
                )
                | (
                    ArtifactKey::ProgramIndex { .. },
                    ArtifactPayload::ProgramIndex(_)
                )
                | (ArtifactKey::Script { .. }, ArtifactPayload::Script(_))
                | (ArtifactKey::Object { .. }, ArtifactPayload::Object(_))
                | (ArtifactKey::Asset { .. }, ArtifactPayload::Asset(_))
                | (ArtifactKey::Build { .. }, ArtifactPayload::Build(_))
                | (ArtifactKey::Bundle { .. }, ArtifactPayload::Bundle(_))
                | (ArtifactKey::Program { .. }, ArtifactPayload::Program(_))
                | (ArtifactKey::Product { .. }, ArtifactPayload::Product(_))
                | (
                    ArtifactKey::ModuleLinted { .. },
                    ArtifactPayload::ModuleLinted(_)
                )
                | (
                    ArtifactKey::PackageLinted { .. },
                    ArtifactPayload::PackageLinted(_)
                )
                | (
                    ArtifactKey::WorkspaceLinted,
                    ArtifactPayload::WorkspaceLinted(_)
                )
        )
    }

    /// Borrow this payload for transport serialization.
    pub fn as_ref(&self) -> ArtifactPayloadRef<'_> {
        match self {
            Self::DirParsed(payload) => ArtifactPayloadRef::DirParsed(payload.as_ref()),
            Self::Data(payload) => ArtifactPayloadRef::Data(payload.as_ref()),
            Self::GlobalEnvironment(payload) => {
                ArtifactPayloadRef::GlobalEnvironment(payload.as_ref())
            }
            Self::PackageIndex(payload) => ArtifactPayloadRef::PackageIndex(payload.as_ref()),
            Self::ComponentGraph(payload) => ArtifactPayloadRef::ComponentGraph(payload.as_ref()),
            Self::ProgramAnalysis(payload) => ArtifactPayloadRef::ProgramAnalysis(payload.as_ref()),
            Self::DirBound(payload) => ArtifactPayloadRef::DirBound(payload.as_ref()),
            Self::DirImported(payload) => ArtifactPayloadRef::DirImported(payload.as_ref()),
            Self::DirExpanded(payload) => ArtifactPayloadRef::DirExpanded(payload.as_ref()),
            Self::DirExported(payload) => ArtifactPayloadRef::DirExported(payload.as_ref()),
            Self::DirResolved(payload) => ArtifactPayloadRef::DirResolved(payload.as_ref()),
            Self::DirCheckedComponent(payload) => {
                ArtifactPayloadRef::DirCheckedComponent(payload.as_ref())
            }
            Self::DirChecked(payload) => ArtifactPayloadRef::DirChecked(payload.as_ref()),
            Self::DirMaterialized(payload) => ArtifactPayloadRef::DirMaterialized(payload.as_ref()),
            Self::MirLowered(payload) => ArtifactPayloadRef::MirLowered(payload.as_ref()),
            Self::MirVerified(payload) => ArtifactPayloadRef::MirVerified(payload.as_ref()),
            Self::MirElaborated(payload) => ArtifactPayloadRef::MirElaborated(payload.as_ref()),
            Self::MirAnalyzed(payload) => ArtifactPayloadRef::MirAnalyzed(payload.as_ref()),
            Self::MirOptimized(payload) => ArtifactPayloadRef::MirOptimized(payload.as_ref()),
            Self::ModuleIndex(payload) => ArtifactPayloadRef::ModuleIndex(payload.as_ref()),
            Self::ProgramIndex(payload) => ArtifactPayloadRef::ProgramIndex(payload.as_ref()),
            Self::Script(payload) => ArtifactPayloadRef::Script(payload.as_ref()),
            Self::Object(payload) => ArtifactPayloadRef::Object(payload.as_ref()),
            Self::Asset(payload) => ArtifactPayloadRef::Asset(payload.as_ref()),
            Self::Build(payload) => ArtifactPayloadRef::Build(payload.as_ref()),
            Self::Bundle(payload) => ArtifactPayloadRef::Bundle(payload.as_ref()),
            Self::Program(payload) => ArtifactPayloadRef::Program(payload.as_ref()),
            Self::Product(payload) => ArtifactPayloadRef::Product(payload.as_ref()),
            Self::ModuleLinted(payload) => ArtifactPayloadRef::ModuleLinted(payload.as_ref()),
            Self::PackageLinted(payload) => ArtifactPayloadRef::PackageLinted(payload.as_ref()),
            Self::WorkspaceLinted(payload) => ArtifactPayloadRef::WorkspaceLinted(payload.as_ref()),
        }
    }

    /// Return the stable fingerprint of one projected payload value.
    pub fn projection_fingerprint(
        &self,
        projection: ArtifactProjectionKey,
    ) -> Option<ArtifactProjectionFingerprint> {
        match (self, projection) {
            (Self::ComponentGraph(payload), ArtifactProjectionKey::ComponentGraph(projection)) => {
                Some(payload.projection_fingerprint(projection))
            }
            (Self::DirCheckedComponent(payload), ArtifactProjectionKey::DirChecked(module)) => {
                payload.module(module).map(|entry| entry.fingerprint)
            }
            (Self::ModuleIndex(payload), ArtifactProjectionKey::ModuleIndex(projection)) => {
                Some(payload.projection_fingerprint(projection))
            }
            _ => None,
        }
    }

    /// Return the stable short name for this payload kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment(_) => "global_environment",
            Self::PackageIndex(_) => "package_index",
            Self::ComponentGraph(_) => "component_graph",
            Self::ProgramAnalysis(_) => "program_analysis",
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
            Self::MirLowered(_) => "mir_lowered",
            Self::MirVerified(_) => "mir_verified",
            Self::MirElaborated(_) => "mir_elaborated",
            Self::MirAnalyzed(_) => "mir_analyzed",
            Self::MirOptimized(_) => "mir_optimized",
            Self::ModuleIndex(_) => "module_index",
            Self::ProgramIndex(_) => "program_index",
            Self::Script(_) => "script",
            Self::Object(_) => "object",
            Self::Asset(_) => "asset",
            Self::Build(_) => "build",
            Self::Bundle(_) => "bundle",
            Self::Program(_) => "program",
            Self::Product(_) => "product",
            Self::ModuleLinted(_) => "module_linted",
            Self::PackageLinted(_) => "package_linted",
            Self::WorkspaceLinted(_) => "workspace_linted",
        }
    }

    /// Return all content ids referenced by this payload.
    pub fn content_ids(&self) -> Vec<ContentId> {
        match self {
            Self::Object(payload) => payload.content_ids(),
            Self::Asset(payload) => payload.content_ids(),
            Self::Build(payload) => payload.content_ids(),
            Self::Bundle(payload) => payload.content_ids(),
            Self::Program(payload) => payload.content_ids(),
            Self::Product(payload) => payload.content_ids(),
            _ => Vec::new(),
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

impl From<ComponentGraph> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ComponentGraph) -> Self {
        Self::ComponentGraph(Arc::new(payload))
    }
}

impl From<ProgramAnalysis> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ProgramAnalysis) -> Self {
        Self::ProgramAnalysis(Arc::new(payload))
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

impl From<MirElaborated> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirElaborated) -> Self {
        Self::MirElaborated(Arc::new(payload))
    }
}

impl From<MirAnalyzed> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirAnalyzed) -> Self {
        Self::MirAnalyzed(Arc::new(payload))
    }
}

impl From<MirOptimized> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirOptimized) -> Self {
        Self::MirOptimized(Arc::new(payload))
    }
}

impl From<ModuleIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleIndex) -> Self {
        Self::ModuleIndex(Arc::new(payload))
    }
}

impl From<ProgramIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ProgramIndex) -> Self {
        Self::ProgramIndex(Arc::new(payload))
    }
}

impl From<Script> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Script) -> Self {
        Self::Script(Arc::new(payload))
    }
}

impl From<Object> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Object) -> Self {
        Self::Object(Arc::new(payload))
    }
}

impl From<Asset> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Asset) -> Self {
        Self::Asset(Arc::new(payload))
    }
}

impl From<Build> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Build) -> Self {
        Self::Build(Arc::new(payload))
    }
}

impl From<Bundle> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Bundle) -> Self {
        Self::Bundle(Arc::new(payload))
    }
}

impl From<Program> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Program) -> Self {
        Self::Program(Arc::new(payload))
    }
}

impl From<Product> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Product) -> Self {
        Self::Product(Arc::new(payload))
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
