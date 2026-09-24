use destack_program::Object;
use destack_serde::Schema;

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactPayload, ArtifactReference, ArtifactVersion, Asset,
    Build, BuildLinkage, BuildManifest, BuildProfile, Bundle, BundleFile, BundleMode,
    BundleSection, Code, Data, DirAnalyzed, DirBound, DirChecked, DirDeclared, DirElaborated,
    DirExpanded, DirExported, DirImported, DirMaterialized, DirParsed, DirResolved,
    EnvironmentBound, EnvironmentDeclared, Host, LanguageEnvironment, LanguageIntrinsics,
    MirAnalyzed, MirDeclared, MirElaborated, MirInstantiated, MirLowered, MirOptimized,
    MirVerified, ModuleEdges, ModuleGraph, ModuleIndex, ModuleLinted, Output, Platform, Product,
    ProductTarget, ProgramAnalysis, ProgramIndex, ProgramLinted, Runtime, Script, SourceMap,
};

/// Include public artifact schema roots.
pub fn schema(schema: &mut Schema) {
    schema.register::<ArtifactKey>();
    schema.register::<ArtifactVersion>();
    schema.register::<ArtifactDependency>();
    schema.register::<ArtifactPayload>();
    schema.register::<ArtifactReference>();

    schema.register::<BuildProfile>();
    schema.register::<BuildLinkage>();
    schema.register::<Output>();
    schema.register::<Code>();
    schema.register::<Runtime>();
    schema.register::<Host>();
    schema.register::<Platform>();
    schema.register::<Build>();

    schema.register::<DirParsed>();
    schema.register::<Data>();
    schema.register::<DirBound>();

    schema.register::<LanguageEnvironment>();
    schema.register::<LanguageIntrinsics>();
    schema.register::<EnvironmentBound>();
    schema.register::<ModuleEdges>();
    schema.register::<ModuleGraph>();

    schema.register::<DirImported>();
    schema.register::<DirExpanded>();
    schema.register::<DirExported>();
    schema.register::<DirResolved>();
    schema.register::<DirDeclared>();
    schema.register::<EnvironmentDeclared>();
    schema.register::<DirElaborated>();
    schema.register::<DirChecked>();
    schema.register::<DirMaterialized>();
    schema.register::<DirAnalyzed>();

    schema.register::<MirDeclared>();
    schema.register::<MirLowered>();
    schema.register::<MirVerified>();
    schema.register::<MirInstantiated>();
    schema.register::<MirElaborated>();
    schema.register::<MirAnalyzed>();
    schema.register::<MirOptimized>();
    schema.register::<ProgramAnalysis>();

    schema.register::<ModuleIndex>();
    schema.register::<ProgramIndex>();
    schema.register::<ModuleLinted>();
    schema.register::<ProgramLinted>();
    schema.register::<BuildManifest>();

    schema.register::<SourceMap>();
    schema.register::<Object>();
    schema.register::<Script>();
    schema.register::<Asset>();
    schema.register::<BundleSection>();
    schema.register::<BundleMode>();
    schema.register::<BundleFile>();
    schema.register::<Bundle>();
    schema.register::<ProductTarget>();
    schema.register::<Product>();
}
