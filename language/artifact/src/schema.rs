use destack_serde::SchemaRegistry;

use crate::{
    ArtifactDependency, ArtifactEventLog, ArtifactKey, ArtifactPayload, ArtifactRecord,
    ArtifactReference, ArtifactSidecar, ArtifactVersion, Asset, Build, BuildLinkage, BuildManifest,
    BuildProfile, Bundle, BundleFile, BundleMode, BundleSection, ComponentGraph, Data, Declaration,
    DirBound, DirChecked, DirCheckedComponent, DirElaborated, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, EmitFormat, GlobalEnvironment, Host,
    LanguageEnvironment, LanguageIntrinsics, MirAnalyzed, MirLowered, MirOptimized, MirVerified,
    ModuleGraph, ModuleIndex, ModuleLinted, Object, ObjectFormat, PackageIndex, PackageLinted,
    Platform, Product, ProductTarget, ProgramAnalysis, ProgramIndex, Runtime, Script, ScriptBody,
    ScriptLanguage, SourceMap, WorkspaceLinted,
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

    registry.register::<Data>();
    registry.register::<ModuleGraph>();
    registry.register::<ComponentGraph>();
    registry.register::<LanguageEnvironment>();
    registry.register::<GlobalEnvironment>();
    registry.register::<LanguageIntrinsics>();
    registry.register::<ArtifactEventLog>();
    registry.register::<ModuleLinted>();
    registry.register::<PackageLinted>();
    registry.register::<WorkspaceLinted>();
    registry.register::<BuildManifest>();
    registry.register::<PackageIndex>();
    registry.register::<ModuleIndex>();
    registry.register::<ProgramIndex>();

    registry.register::<DirParsed>();
    registry.register::<DirBound>();
    registry.register::<DirImported>();
    registry.register::<DirExpanded>();
    registry.register::<DirExported>();
    registry.register::<DirResolved>();
    registry.register::<DirCheckedComponent>();
    registry.register::<DirChecked>();
    registry.register::<DirMaterialized>();
    registry.register::<DirElaborated>();

    registry.register::<MirLowered>();
    registry.register::<MirVerified>();
    registry.register::<MirOptimized>();
    registry.register::<MirAnalyzed>();
    registry.register::<ProgramAnalysis>();

    registry.register::<SourceMap>();
    registry.register::<ObjectFormat>();
    registry.register::<Object>();
    registry.register::<ScriptLanguage>();
    registry.register::<Declaration>();
    registry.register::<ScriptBody>();
    registry.register::<Script>();
    registry.register::<Asset>();
    registry.register::<Build>();
    registry.register::<BundleSection>();
    registry.register::<BundleMode>();
    registry.register::<BundleFile>();
    registry.register::<Bundle>();
    registry.register::<ProductTarget>();
    registry.register::<Product>();
}
