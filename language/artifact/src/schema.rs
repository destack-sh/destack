use destack_serde::SchemaRegistry;

use crate::{
    ArtifactDependency, ArtifactEventLog, ArtifactKey, ArtifactPayload, ArtifactRecord,
    ArtifactReference, ArtifactSidecar, ArtifactVersion, Asset, Build, BuildLinkage, BuildManifest,
    BuildProfile, Bundle, BundleFile, BundleMode, BundleSection, ComponentGraph, Data, DirBound,
    DirChecked, DirCheckedComponent, DirExpanded, DirExported, DirImported, DirMaterialized,
    DirParsed, DirResolved, EmitFormat, GlobalEnvironment, Host, LanguageEnvironment,
    LanguageIntrinsics, MirAnalyzed, MirElaborated, MirLowered, MirOptimized, MirVerified,
    ModuleGraph, ModuleIndex, ModuleLinted, Object, ObjectFormat, PackageGraph, Platform, Product,
    ProductTarget, ProgramAnalysis, ProgramIndex, ProgramLinted, Runtime, Script, ScriptBody,
    ScriptLanguage, SourceMap,
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
    registry.register::<ProgramLinted>();
    registry.register::<BuildManifest>();
    registry.register::<PackageGraph>();
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

    registry.register::<MirLowered>();
    registry.register::<MirVerified>();
    registry.register::<MirElaborated>();
    registry.register::<MirOptimized>();
    registry.register::<MirAnalyzed>();
    registry.register::<ProgramAnalysis>();

    registry.register::<SourceMap>();
    registry.register::<ObjectFormat>();
    registry.register::<Object>();
    registry.register::<ScriptLanguage>();
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
