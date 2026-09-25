use std::borrow::Cow;
use std::sync::Arc;

use tspp_core::Blob;
use tspp_program::{Object, Program};
use tspp_serde as serde;
use tspp_serde::Reflect;
use tspp_source::{ModuleId, PackageId, ProductId, ProfileId, TargetId};

use crate::{
    ArtifactError, ArtifactKey, ArtifactProjectionFingerprint, ArtifactProjectionKey, Asset, Build,
    Bundle, Data, DirAnalyzed, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded,
    DirExported, DirImported, DirMaterialized, DirParsed, DirResolved, EnvironmentBound,
    EnvironmentDeclared, IndexKind, MirAnalyzed, MirDeclared, MirElaborated, MirInstantiated,
    MirLowered, MirOptimized, MirVerified, ModuleGraph, ModuleIndex, ModuleLinted, Product,
    ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
};

use ::serde::{Deserialize, Serialize};

/// One typed artifact payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum ArtifactPayload {
    /// Target-built toolchain payload.
    Build(Arc<Build>),
    /// Parsed module DIR.
    DirParsed(Arc<DirParsed>),
    /// Parsed non-code module data.
    Data(Arc<Data>),
    /// Bound DIR.
    DirBound(Arc<DirBound>),
    /// One bound environment payload.
    EnvironmentBound(Arc<EnvironmentBound>),
    /// Component partition for one profile.
    ModuleGraph(Arc<ModuleGraph>),
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
    /// One declared environment payload.
    EnvironmentDeclared(Arc<EnvironmentDeclared>),
    /// Elaborated DIR module.
    DirElaborated(Arc<DirElaborated>),
    /// Checked DIR module.
    DirChecked(Arc<DirChecked>),
    /// Materialized DIR.
    DirMaterialized(Arc<DirMaterialized>),
    /// Analyzed DIR.
    DirAnalyzed(Arc<DirAnalyzed>),
    /// The declarations one module lowers ahead of its bodies.
    MirDeclared(Arc<MirDeclared>),
    /// Lowered MIR before optimization.
    MirLowered(Arc<MirLowered>),
    /// Verified MIR marker after required semantic verification.
    MirVerified(Arc<MirVerified>),
    /// Instantiated MIR.
    MirInstantiated(Arc<MirInstantiated>),
    /// MIR after required elaboration.
    MirElaborated(Arc<MirElaborated>),
    /// Per-module link graph for whole-program analysis.
    MirAnalyzed(Arc<MirAnalyzed>),
    /// Optimized MIR.
    MirOptimized(Arc<MirOptimized>),
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis(Arc<ProgramAnalysis>),
    /// One index for a module profile.
    ModuleIndex(Arc<ModuleIndex>),
    /// One index for a program profile.
    ProgramIndex(Arc<ProgramIndex>),
    /// Completed lint analysis for one module in one target.
    ModuleLinted(Arc<ModuleLinted>),
    /// Completed lint analysis for one target program.
    ProgramLinted(Arc<ProgramLinted>),
    /// One structured linker input for one target.
    Script(Arc<Script>),
    /// One optimized module object for one target.
    Object(Arc<Object>),
    /// One opaque linker input for one target.
    Asset(Arc<Asset>),
    /// Linked file graph for one package target.
    Bundle(Arc<Bundle>),
    /// Program for one package target.
    Program(Arc<Program>),
    /// Linked product assembled from configured target artifacts.
    Product(Arc<Product>),
}

/// One borrowed typed artifact payload.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum ArtifactPayloadRef<'a> {
    /// Target-built toolchain payload.
    Build(&'a Build),
    /// Parsed module DIR.
    DirParsed(&'a DirParsed),
    /// Parsed non-code module data.
    Data(&'a Data),
    /// Bound DIR.
    DirBound(&'a DirBound),
    /// One bound environment payload.
    EnvironmentBound(&'a EnvironmentBound),
    /// Component partition for one profile.
    ModuleGraph(&'a ModuleGraph),
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
    /// One declared environment payload.
    EnvironmentDeclared(&'a EnvironmentDeclared),
    /// Elaborated DIR module.
    DirElaborated(&'a DirElaborated),
    /// Checked DIR module.
    DirChecked(&'a DirChecked),
    /// Materialized DIR.
    DirMaterialized(&'a DirMaterialized),
    /// Analyzed DIR.
    DirAnalyzed(&'a DirAnalyzed),
    /// The declarations one module lowers ahead of its bodies.
    MirDeclared(&'a MirDeclared),
    /// Lowered MIR before optimization.
    MirLowered(&'a MirLowered),
    /// Verified MIR marker after required semantic verification.
    MirVerified(&'a MirVerified),
    /// Instantiated MIR.
    MirInstantiated(&'a MirInstantiated),
    /// MIR after required elaboration.
    MirElaborated(&'a MirElaborated),
    /// Per-module link graph for whole-program analysis.
    MirAnalyzed(&'a MirAnalyzed),
    /// Optimized MIR.
    MirOptimized(&'a MirOptimized),
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis(&'a ProgramAnalysis),
    /// One index for a module profile.
    ModuleIndex(&'a ModuleIndex),
    /// One index for a program profile.
    ProgramIndex(&'a ProgramIndex),
    /// Completed lint analysis for one module in one target.
    ModuleLinted(&'a ModuleLinted),
    /// Completed lint analysis for one target program.
    ProgramLinted(&'a ProgramLinted),
    /// One structured linker input for one target.
    Script(&'a Script),
    /// One optimized module object for one target.
    Object(&'a Object),
    /// One opaque linker input for one target.
    Asset(&'a Asset),
    /// Linked file graph for one package target.
    Bundle(&'a Bundle),
    /// Program for one package target.
    Program(&'a Program),
    /// Linked product assembled from configured target artifacts.
    Product(&'a Product),
}

/// One typed artifact stored under a typed key.
pub trait Artifact: Sized {
    /// The values that identify one artifact of this type.
    type Key;

    /// Convert one typed key into its repository identity.
    fn artifact_key(key: Self::Key) -> ArtifactKey;

    /// Extract this artifact type from one stored payload.
    fn from_payload(payload: ArtifactPayload) -> Option<Arc<Self>>;
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
                    ArtifactKey::EnvironmentBound { .. },
                    ArtifactPayload::EnvironmentBound(_)
                ) | (
                    ArtifactKey::EnvironmentDeclared { .. },
                    ArtifactPayload::EnvironmentDeclared(_)
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
                        ArtifactKey::DirElaborated { .. },
                        ArtifactPayload::DirElaborated(_)
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
                        ArtifactKey::DirAnalyzed { .. },
                        ArtifactPayload::DirAnalyzed(_)
                    )
                    | (
                        ArtifactKey::MirDeclared { .. },
                        ArtifactPayload::MirDeclared(_)
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
                        ArtifactKey::MirInstantiated { .. },
                        ArtifactPayload::MirInstantiated(_)
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
            Self::EnvironmentBound(payload) => {
                ArtifactPayloadRef::EnvironmentBound(payload.as_ref())
            }
            Self::EnvironmentDeclared(payload) => {
                ArtifactPayloadRef::EnvironmentDeclared(payload.as_ref())
            }
            Self::ModuleGraph(payload) => ArtifactPayloadRef::ModuleGraph(payload.as_ref()),
            Self::ProgramAnalysis(payload) => ArtifactPayloadRef::ProgramAnalysis(payload.as_ref()),
            Self::DirBound(payload) => ArtifactPayloadRef::DirBound(payload.as_ref()),
            Self::DirImported(payload) => ArtifactPayloadRef::DirImported(payload.as_ref()),
            Self::DirExpanded(payload) => ArtifactPayloadRef::DirExpanded(payload.as_ref()),
            Self::DirExported(payload) => ArtifactPayloadRef::DirExported(payload.as_ref()),
            Self::DirResolved(payload) => ArtifactPayloadRef::DirResolved(payload.as_ref()),
            Self::DirDeclared(payload) => ArtifactPayloadRef::DirDeclared(payload.as_ref()),
            Self::DirElaborated(payload) => ArtifactPayloadRef::DirElaborated(payload.as_ref()),
            Self::DirChecked(payload) => ArtifactPayloadRef::DirChecked(payload.as_ref()),
            Self::DirMaterialized(payload) => ArtifactPayloadRef::DirMaterialized(payload.as_ref()),
            Self::DirAnalyzed(payload) => ArtifactPayloadRef::DirAnalyzed(payload.as_ref()),
            Self::MirDeclared(payload) => ArtifactPayloadRef::MirDeclared(payload.as_ref()),
            Self::MirLowered(payload) => ArtifactPayloadRef::MirLowered(payload.as_ref()),
            Self::MirVerified(payload) => ArtifactPayloadRef::MirVerified(payload.as_ref()),
            Self::MirInstantiated(payload) => ArtifactPayloadRef::MirInstantiated(payload.as_ref()),
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
            Self::EnvironmentBound(_) => "environment_bound",
            Self::EnvironmentDeclared(_) => "environment_declared",
            Self::ModuleGraph(_) => "module_graph",
            Self::ProgramAnalysis(_) => "program_analysis",
            Self::DirParsed(_) => "dir_parsed",
            Self::Data(_) => "data",
            Self::DirBound(_) => "dir_bound",
            Self::DirImported(_) => "dir_imported",
            Self::DirExpanded(_) => "dir_expanded",
            Self::DirExported(_) => "dir_exported",
            Self::DirResolved(_) => "dir_resolved",
            Self::DirDeclared(_) => "dir_declared",
            Self::DirElaborated(_) => "dir_elaborated",
            Self::DirChecked(_) => "dir_checked",
            Self::DirMaterialized(_) => "dir_materialized",
            Self::DirAnalyzed(_) => "dir_analyzed",
            Self::MirDeclared(_) => "mir_declared",
            Self::MirLowered(_) => "mir_lowered",
            Self::MirVerified(_) => "mir_verified",
            Self::MirInstantiated(_) => "mir_instantiated",
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

    /// Return every Blob referenced by this payload.
    pub fn blobs(&self) -> Vec<Blob> {
        self.as_ref().blobs()
    }
}

impl<'a> ArtifactPayloadRef<'a> {
    /// Return every Blob referenced by this payload.
    pub fn blobs(self) -> Vec<Blob> {
        match self {
            Self::Asset(payload) => payload.blobs(),
            Self::Build(payload) => payload.blobs(),
            Self::Bundle(payload) => payload.blobs(),
            _ => Vec::new(),
        }
    }

    /// Encode this payload for artifact storage.
    pub fn encode(self) -> Result<Cow<'a, [u8]>, ArtifactError> {
        match self {
            Self::Program(program) => Ok(Cow::Borrowed(program.bytes())),
            payload => serde::to_vec(&payload)
                .map(Cow::Owned)
                .map_err(|error| ArtifactError::Codec(Box::new(error))),
        }
    }

    /// Return one observable projection fingerprint for this payload.
    pub(crate) fn fingerprint_projection(
        self,
        projection: ArtifactProjectionKey,
    ) -> Result<Option<ArtifactProjectionFingerprint>, ArtifactError> {
        let fingerprint = match (self, projection) {
            (Self::DirDeclared(payload), ArtifactProjectionKey::Payload) => {
                Some(payload.fingerprint)
            }
            (Self::DirElaborated(payload), ArtifactProjectionKey::Payload) => {
                Some(payload.fingerprint)
            }
            (Self::DirChecked(payload), ArtifactProjectionKey::Payload) => {
                Some(payload.fingerprint)
            }
            (payload, ArtifactProjectionKey::Payload) => Some(
                ArtifactProjectionFingerprint::from_serialized_payload(&payload)?,
            ),
            (
                Self::ProgramAnalysis(payload),
                ArtifactProjectionKey::ProgramAnalysisFunctionEffects(symbol),
            ) => Some(ArtifactProjectionFingerprint::from_serialized_payload(
                &payload.effects.function(symbol),
            )?),
            (Self::ModuleGraph(payload), projection) => payload.fingerprint_projection(projection),
            (Self::DirResolved(payload), ArtifactProjectionKey::DirResolvedComponentRelations) => {
                Some(payload.component_relations_fingerprint())
            }
            _ => None,
        };

        Ok(fingerprint)
    }
}

macro_rules! artifact {
    ($type:ty, $variant:ident, $key:ty, $key_pattern:pat => $artifact_key:expr) => {
        impl Artifact for $type {
            type Key = $key;

            fn artifact_key($key_pattern: Self::Key) -> ArtifactKey {
                $artifact_key
            }

            fn from_payload(payload: ArtifactPayload) -> Option<Arc<Self>> {
                match payload {
                    ArtifactPayload::$variant(payload) => Some(payload),
                    _ => None,
                }
            }
        }

        impl From<$type> for ArtifactPayload {
            /// Convert a typed artifact into an artifact payload.
            fn from(payload: $type) -> Self {
                Self::$variant(Arc::new(payload))
            }
        }
    };
}

artifact!(Build, Build, TargetId, target => ArtifactKey::build(target));
artifact!(DirParsed, DirParsed, ModuleId, module => ArtifactKey::dir_parsed(module));
artifact!(Data, Data, ModuleId, module => ArtifactKey::data(module));
artifact!(
    DirBound,
    DirBound,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_bound(module, profile)
);
artifact!(
    EnvironmentBound,
    EnvironmentBound,
    ProfileId,
    profile => ArtifactKey::environment_bound(profile)
);
artifact!(
    ModuleGraph,
    ModuleGraph,
    (PackageId, ProfileId),
    (package, profile) => ArtifactKey::module_graph(package, profile)
);
artifact!(
    DirImported,
    DirImported,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_imported(module, profile)
);
artifact!(
    DirExpanded,
    DirExpanded,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_expanded(module, profile)
);
artifact!(
    DirExported,
    DirExported,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_exported(module, profile)
);
artifact!(
    DirResolved,
    DirResolved,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_resolved(module, profile)
);
artifact!(
    DirDeclared,
    DirDeclared,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_declared(module, profile)
);
artifact!(
    EnvironmentDeclared,
    EnvironmentDeclared,
    ProfileId,
    profile => ArtifactKey::environment_declared(profile)
);
artifact!(
    DirElaborated,
    DirElaborated,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_elaborated(module, profile)
);
artifact!(
    DirChecked,
    DirChecked,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_checked(module, profile)
);
artifact!(
    DirAnalyzed,
    DirAnalyzed,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_analyzed(module, profile)
);
artifact!(
    DirMaterialized,
    DirMaterialized,
    (ModuleId, ProfileId),
    (module, profile) => ArtifactKey::dir_materialized(module, profile)
);
artifact!(
    MirDeclared,
    MirDeclared,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_declared(module, profile, target)
);
artifact!(
    MirLowered,
    MirLowered,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_lowered(module, profile, target)
);
artifact!(
    MirVerified,
    MirVerified,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_verified(module, profile, target)
);
artifact!(
    MirInstantiated,
    MirInstantiated,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_instantiated(module, profile, target)
);
artifact!(
    MirElaborated,
    MirElaborated,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_elaborated(module, profile, target)
);
artifact!(
    MirAnalyzed,
    MirAnalyzed,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_analyzed(module, profile, target)
);
artifact!(
    MirOptimized,
    MirOptimized,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::mir_optimized(module, profile, target)
);
artifact!(
    ProgramAnalysis,
    ProgramAnalysis,
    (ProfileId, TargetId),
    (profile, target) => ArtifactKey::program_analysis(profile, target)
);
artifact!(
    ModuleIndex,
    ModuleIndex,
    (ModuleId, ProfileId, IndexKind),
    (module, profile, kind) => ArtifactKey::module_index(module, profile, kind)
);
artifact!(
    ProgramIndex,
    ProgramIndex,
    (ProfileId, IndexKind),
    (profile, kind) => ArtifactKey::program_index(profile, kind)
);
artifact!(
    ModuleLinted,
    ModuleLinted,
    (ModuleId, ProfileId, TargetId),
    (module, profile, target) => ArtifactKey::module_linted(module, profile, target)
);
artifact!(
    ProgramLinted,
    ProgramLinted,
    (ProfileId, TargetId),
    (profile, target) => ArtifactKey::program_linted(profile, target)
);
artifact!(
    Script,
    Script,
    (ModuleId, TargetId),
    (module, target) => ArtifactKey::script(module, target)
);
artifact!(
    Object,
    Object,
    (ModuleId, TargetId),
    (module, target) => ArtifactKey::object(module, target)
);
artifact!(
    Asset,
    Asset,
    (ModuleId, TargetId),
    (module, target) => ArtifactKey::asset(module, target)
);
artifact!(
    Bundle,
    Bundle,
    (PackageId, TargetId),
    (package, target) => ArtifactKey::bundle(package, target)
);
artifact!(
    Program,
    Program,
    (PackageId, TargetId),
    (package, target) => ArtifactKey::program(package, target)
);
artifact!(
    Product,
    Product,
    (PackageId, ProductId),
    (package, product) => ArtifactKey::product(package, product)
);
