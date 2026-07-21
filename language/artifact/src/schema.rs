use destack_serde::SchemaRegistry;

use crate::{
    ArtifactDependency, ArtifactEventLog, ArtifactKey, ArtifactPayload, ArtifactRecord,
    ArtifactReference, ArtifactSidecar, ArtifactVersion, Asset, Build, BuildLinkage, BuildManifest,
    BuildProfile, Bundle, BundleFile, BundleMode, BundleSection, Data, DirBound, DirChecked,
    DirDeclared, DirExpanded, DirExported, DirImported, DirMaterialized, DirParsed, DirResolved,
    EmitFormat, EnvironmentBound, EnvironmentDeclared, Host, LanguageEnvironment,
    LanguageIntrinsics, MirAnalyzed, MirElaborated, MirLowered, MirOptimized, MirVerified,
    ModuleEdges, ModuleGraph, ModuleIndex, ModuleLinted, Object, Platform, Product, ProductTarget,
    ProgramAnalysis, ProgramIndex, ProgramLinted, Runtime, Script, ScriptBody, ScriptLanguage,
    SourceMap,
};

/// Include public artifact schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<ArtifactKey>();
    registry.register::<ArtifactVersion>();
    registry.register::<ArtifactDependency>();
    registry.register::<ArtifactSidecar>();
    registry.register::<ArtifactRecord>();
    registry.register::<ArtifactPayload>();
    registry.register::<ArtifactReference>();

    registry.register::<BuildProfile>();
    registry.register::<BuildLinkage>();
    registry.register::<EmitFormat>();
    registry.register::<Runtime>();
    registry.register::<Host>();
    registry.register::<Platform>();
    registry.register::<Build>();

    registry.register::<DirParsed>();
    registry.register::<Data>();
    registry.register::<DirBound>();

    registry.register::<LanguageEnvironment>();
    registry.register::<LanguageIntrinsics>();
    registry.register::<EnvironmentBound>();
    registry.register::<ModuleEdges>();
    registry.register::<ModuleGraph>();

    registry.register::<DirImported>();
    registry.register::<DirExpanded>();
    registry.register::<DirExported>();
    registry.register::<DirResolved>();
    registry.register::<DirDeclared>();
    registry.register::<EnvironmentDeclared>();
    registry.register::<DirChecked>();
    registry.register::<DirMaterialized>();

    registry.register::<MirLowered>();
    registry.register::<MirVerified>();
    registry.register::<MirElaborated>();
    registry.register::<MirAnalyzed>();
    registry.register::<MirOptimized>();
    registry.register::<ProgramAnalysis>();

    registry.register::<ModuleIndex>();
    registry.register::<ProgramIndex>();
    registry.register::<ModuleLinted>();
    registry.register::<ProgramLinted>();
    registry.register::<ArtifactEventLog>();
    registry.register::<BuildManifest>();

    registry.register::<SourceMap>();
    registry.register::<Object>();
    registry.register::<ScriptLanguage>();
    registry.register::<ScriptBody>();
    registry.register::<Script>();
    registry.register::<Asset>();
    registry.register::<BundleSection>();
    registry.register::<BundleMode>();
    registry.register::<BundleFile>();
    registry.register::<Bundle>();
    registry.register::<ProductTarget>();
    registry.register::<Product>();
}
