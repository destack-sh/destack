use std::sync::Arc;

use destack_program::Program;
use destack_serde::Reflect;
use destack_source::ContentId;

use crate::{
    ArtifactKey, ArtifactProjectionFingerprint, ArtifactProjectionKey, Asset, Build, Bundle, Data,
    DirBound, DirChecked, DirDeclared, DirExpanded, DirExported, DirImported, DirMaterialized,
    DirParsed, DirResolved, GlobalEnvironment, MirAnalyzed, MirElaborated, MirLowered,
    MirOptimized, MirVerified, ModuleGraph, ModuleIndex, ModuleLinted, Object, Product,
    ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
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
    /// Component partition for one profile.
    ModuleGraph(Arc<ModuleGraph>),
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
    /// Declared DIR module.
    DirDeclared(Arc<DirDeclared>),
    /// Checked DIR module.
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
    /// One query index for a module profile.
    ModuleIndex(Arc<ModuleIndex>),
    /// One query index for a program profile.
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
    /// Completed lint analysis for one module in one target.
    ModuleLinted(Arc<ModuleLinted>),
    /// Completed lint analysis for one target program.
    ProgramLinted(Arc<ProgramLinted>),
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
    /// Component partition for one profile.
    ModuleGraph(&'a ModuleGraph),
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
    /// Declared DIR module.
    DirDeclared(&'a DirDeclared),
    /// Checked DIR module.
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
    /// One query index for a module profile.
    ModuleIndex(&'a ModuleIndex),
    /// One query index for a program profile.
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
    /// Completed lint analysis for one module in one target.
    ModuleLinted(&'a ModuleLinted),
    /// Completed lint analysis for one target program.
    ProgramLinted(&'a ProgramLinted),
}

impl ArtifactPayload {
    /// Return whether this payload belongs to one artifact key.
    pub fn matches_key(&self, key: &ArtifactKey) -> bool {
        match (key, self) {
            (ArtifactKey::ModuleIndex { kind, .. }, ArtifactPayload::ModuleIndex(index)) => {
                *kind == index.kind()
            }
            (ArtifactKey::ProgramIndex { kind, .. }, ArtifactPayload::ProgramIndex(index)) => {
                *kind == index.kind()
            }
            (key, payload) => matches!(
                (key, payload),
                (
                    ArtifactKey::GlobalEnvironment { .. },
                    ArtifactPayload::GlobalEnvironment(_)
                ) | (
                    ArtifactKey::ModuleGraph { .. },
                    ArtifactPayload::ModuleGraph(_)
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
                        ArtifactKey::DirDeclared { .. },
                        ArtifactPayload::DirDeclared(_)
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
                        ArtifactKey::ProgramLinted { .. },
                        ArtifactPayload::ProgramLinted(_)
                    )
            ),
        }
    }

    /// Borrow this payload for transport serialization.
    pub fn as_ref(&self) -> ArtifactPayloadRef<'_> {
        match self {
            Self::DirParsed(payload) => ArtifactPayloadRef::DirParsed(payload.as_ref()),
            Self::Data(payload) => ArtifactPayloadRef::Data(payload.as_ref()),
            Self::GlobalEnvironment(payload) => {
                ArtifactPayloadRef::GlobalEnvironment(payload.as_ref())
            }
            Self::ModuleGraph(payload) => ArtifactPayloadRef::ModuleGraph(payload.as_ref()),
            Self::ProgramAnalysis(payload) => ArtifactPayloadRef::ProgramAnalysis(payload.as_ref()),
            Self::DirBound(payload) => ArtifactPayloadRef::DirBound(payload.as_ref()),
            Self::DirImported(payload) => ArtifactPayloadRef::DirImported(payload.as_ref()),
            Self::DirExpanded(payload) => ArtifactPayloadRef::DirExpanded(payload.as_ref()),
            Self::DirExported(payload) => ArtifactPayloadRef::DirExported(payload.as_ref()),
            Self::DirResolved(payload) => ArtifactPayloadRef::DirResolved(payload.as_ref()),
            Self::DirDeclared(payload) => ArtifactPayloadRef::DirDeclared(payload.as_ref()),
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
            Self::ProgramLinted(payload) => ArtifactPayloadRef::ProgramLinted(payload.as_ref()),
        }
    }

    /// Return the stable short name for this payload kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment(_) => "global_environment",
            Self::ModuleGraph(_) => "component_graph",
            Self::ProgramAnalysis(_) => "program_analysis",
            Self::DirParsed(_) => "dir_parsed",
            Self::Data(_) => "data",
            Self::DirBound(_) => "dir_bound",
            Self::DirImported(_) => "dir_imported",
            Self::DirExpanded(_) => "dir_expanded",
            Self::DirExported(_) => "dir_exported",
            Self::DirResolved(_) => "dir_resolved",
            Self::DirDeclared(_) => "dir_declared",
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
            Self::ProgramLinted(_) => "program_linted",
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

impl ArtifactPayloadRef<'_> {
    /// Return one observable projection fingerprint for this payload.
    pub(crate) fn fingerprint_projection(
        self,
        projection: ArtifactProjectionKey,
    ) -> Option<ArtifactProjectionFingerprint> {
        match (self, projection) {
            (Self::ModuleGraph(payload), projection) => payload.fingerprint_projection(projection),
            (Self::DirResolved(payload), ArtifactProjectionKey::ImportEdges) => {
                Some(payload.component_edges_fingerprint())
            }
            (Self::DirDeclared(payload), ArtifactProjectionKey::Declared) => {
                Some(payload.fingerprint)
            }
            (Self::DirChecked(payload), ArtifactProjectionKey::Checked) => {
                Some(payload.fingerprint)
            }
            // hash content directly, ignoring the inputs that rebuilt identical payloads
            (Self::DirBound(payload), ArtifactProjectionKey::Content) => {
                ArtifactProjectionFingerprint::from_serialized_payload(payload).ok()
            }
            (Self::DirExpanded(payload), ArtifactProjectionKey::Content) => {
                ArtifactProjectionFingerprint::from_serialized_payload(payload).ok()
            }
            (Self::DirResolved(payload), ArtifactProjectionKey::Content) => {
                ArtifactProjectionFingerprint::from_serialized_payload(payload).ok()
            }
            (Self::GlobalEnvironment(payload), ArtifactProjectionKey::Content) => {
                ArtifactProjectionFingerprint::from_serialized_payload(payload).ok()
            }
            (Self::DirExported(payload), ArtifactProjectionKey::Content) => {
                ArtifactProjectionFingerprint::from_serialized_payload(payload).ok()
            }
            _ => None,
        }
    }

    /// Return all observable projection fingerprints for this payload.
    pub(crate) fn fingerprint_projections(
        self,
    ) -> Result<Vec<(ArtifactProjectionKey, ArtifactProjectionFingerprint)>, ArtifactProjectionKey>
    {
        let mut projections = Vec::new();

        // index module graph columns at their natural row granularity
        if let Self::ModuleGraph(graph) = self {
            for module in graph.modules() {
                let keys = [ArtifactProjectionKey::ModuleEdges(*module)];
                push_projection_fingerprints(self, keys, &mut projections)?;
            }
            push_projection_fingerprints(
                self,
                [
                    ArtifactProjectionKey::Modules,
                    ArtifactProjectionKey::InherentExtensions,
                ],
                &mut projections,
            )?;
        }

        // index independently reusable resolved DIR relationships
        if matches!(self, Self::DirResolved(_)) {
            push_projection_fingerprints(
                self,
                [ArtifactProjectionKey::ImportEdges],
                &mut projections,
            )?;
        }

        // index independently reusable module outputs
        match self {
            Self::DirDeclared(declared) => {
                projections.push((ArtifactProjectionKey::Declared, declared.fingerprint));
            }
            Self::DirChecked(checked) => {
                projections.push((ArtifactProjectionKey::Checked, checked.fingerprint));
            }
            _ => {}
        }

        // canonicalize and reject duplicate projection keys
        projections.sort_unstable_by_key(|(key, _fingerprint)| *key);
        if let Some(projection) = projections
            .windows(2)
            .find(|entries| entries[0].0 == entries[1].0)
        {
            return Err(projection[0].0);
        }

        Ok(projections)
    }
}

/// Append all present projection fingerprints for one payload.
fn push_projection_fingerprints(
    payload: ArtifactPayloadRef<'_>,
    keys: impl IntoIterator<Item = ArtifactProjectionKey>,
    projections: &mut Vec<(ArtifactProjectionKey, ArtifactProjectionFingerprint)>,
) -> Result<(), ArtifactProjectionKey> {
    for key in keys {
        let Some(fingerprint) = payload.fingerprint_projection(key) else {
            return Err(key);
        };

        projections.push((key, fingerprint));
    }

    Ok(())
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

impl From<ModuleGraph> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleGraph) -> Self {
        Self::ModuleGraph(Arc::new(payload))
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

impl From<DirDeclared> for ArtifactPayload {
    /// Wrap one declared DIR module payload.
    fn from(payload: DirDeclared) -> Self {
        Self::DirDeclared(Arc::new(payload))
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

impl From<ProgramLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ProgramLinted) -> Self {
        Self::ProgramLinted(Arc::new(payload))
    }
}
