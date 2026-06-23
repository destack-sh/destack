use destack_serde::SchemaRegistry;

use crate::{
    Applicability, ArtifactDependency, ArtifactKey, ArtifactRecord, ArtifactSidecar,
    ArtifactSidecarLabel, ArtifactString, ArtifactVersion, Asset, Build, BuildLinkage,
    BuildProfile, Bundle, BundleFile, BundleMode, BundleSection, ComponentId, Content, ContentId,
    Declaration, Diagnostic, DiagnosticHelp, DiagnosticLabel, DiagnosticNote, DiagnosticSeverity,
    DiagnosticSuggestion, DiagnosticTag, DirChecked, DirParsed, DirResolved, EmitFormat, FileId,
    FilePatch, FileType, Host, ModuleId, Object, ObjectFormat, PackageId, Patch, PatchSet, Product,
    ProductId, ProductTarget, ProfileId, Program, ProgramFormat, ProgramHeader, Revision, Runtime,
    Script, ScriptLanguage, SourceMap, SourceMapSource, Span, TargetId, TraceArtifact,
    TraceCounter, TraceReport, TraceSpan, TraceStage, TraceTime,
};

/// Build the public language bridge schema.
pub fn schema() -> SchemaRegistry {
    let mut schema = SchemaRegistry::default();

    // source identity
    schema.include::<Revision>();
    schema.include::<PackageId>();
    schema.include::<ModuleId>();
    schema.include::<ProfileId>();
    schema.include::<ComponentId>();
    schema.include::<TargetId>();
    schema.include::<ProductId>();
    schema.include::<FileId>();
    schema.include::<ContentId>();
    schema.include::<Content>();
    schema.include::<Span>();

    // diagnostics
    schema.include::<DiagnosticSeverity>();
    schema.include::<DiagnosticTag>();
    schema.include::<Applicability>();
    schema.include::<DiagnosticLabel>();
    schema.include::<DiagnosticNote>();
    schema.include::<DiagnosticHelp>();
    schema.include::<DiagnosticSuggestion>();
    schema.include::<Diagnostic>();
    schema.include::<Patch>();
    schema.include::<FilePatch>();
    schema.include::<PatchSet>();

    // artifacts
    schema.include::<ArtifactKey>();
    schema.include::<ArtifactVersion>();
    schema.include::<ArtifactDependency>();
    schema.include::<ArtifactSidecarLabel>();
    schema.include::<ArtifactSidecar>();
    schema.include::<ArtifactString>();
    schema.include::<ArtifactRecord>();

    // build outputs
    schema.include::<BuildProfile>();
    schema.include::<BuildLinkage>();
    schema.include::<EmitFormat>();
    schema.include::<FileType>();
    schema.include::<SourceMapSource>();
    schema.include::<SourceMap>();
    schema.include::<Declaration>();
    schema.include::<ScriptLanguage>();
    schema.include::<Script>();
    schema.include::<ObjectFormat>();
    schema.include::<Object>();
    schema.include::<Asset>();
    schema.include::<Build>();
    schema.include::<BundleSection>();
    schema.include::<BundleMode>();
    schema.include::<BundleFile>();
    schema.include::<Bundle>();
    schema.include::<ProgramFormat>();
    schema.include::<ProgramHeader>();
    schema.include::<Program>();
    schema.include::<Runtime>();
    schema.include::<Host>();
    schema.include::<ProductTarget>();
    schema.include::<Product>();

    // DIR projections
    schema.include::<DirParsed>();
    schema.include::<DirResolved>();
    schema.include::<DirChecked>();

    // traces
    schema.include::<TraceReport>();
    schema.include::<TraceStage>();
    schema.include::<TraceTime>();
    schema.include::<TraceArtifact>();
    schema.include::<TraceSpan>();
    schema.include::<TraceCounter>();

    schema
}
