export { ArtifactKey } from "./artifact/key.generated.js";
export {
    BuildOutput,
    BuildRequest,
} from "./artifact/output.generated.js";
export type {
    Asset,
    Build,
    BuildLinkage,
    BuildProfile,
    Bundle,
    BundleFile,
    BundleMode,
    BundleSection,
    Declaration,
    EmitFormat,
    FileType,
    Host,
    ModuleBuildKind,
    Object,
    ObjectFormat,
    Product,
    ProductTarget,
    Program,
    ProgramFormat,
    ProgramHeader,
    Runtime,
    Script,
    ScriptLanguage,
    SourceMap,
    SourceMapSource,
} from "./artifact/output.generated.js";
export type { ArtifactDependency } from "./artifact/dependency.generated.js";
export type {
    ArtifactRecord,
    ArtifactString,
} from "./artifact/record.generated.js";
export type {
    ArtifactSidecar,
    ArtifactSidecarLabel,
} from "./artifact/sidecar.generated.js";
export type { ArtifactVersion } from "./artifact/version.generated.js";
export type {
    Diagnostic,
    DiagnosticHelp,
    DiagnosticLabel,
    DiagnosticNote,
    DiagnosticSeverity,
    DiagnosticSuggestion,
    DiagnosticTag,
    Applicability,
} from "./diagnostic/diagnostic.generated.js";
export type {
    BatchEdit,
    FilePatch,
    Replacement,
} from "./diagnostic/edit.generated.js";
export type { DirChecked } from "./dir/checked.generated.js";
export type { DirParsed } from "./dir/parsed.generated.js";
export type { DirResolved } from "./dir/resolved.generated.js";
export type { Revision } from "./repository/revision.generated.js";
export type { ComponentId } from "./source/component.generated.js";
export type {
    Content,
    ContentId,
    FileId,
} from "./source/file.generated.js";
export type { ModuleId } from "./source/module.generated.js";
export type { PackageId } from "./source/package.generated.js";
export type { ProductId } from "./source/product.generated.js";
export type { ProfileId } from "./source/profile.generated.js";
export type { Span } from "./source/span.generated.js";
export type { TargetId } from "./source/target.generated.js";
export type { SessionFile } from "./session/file.generated.js";
export type { CheckOutput } from "./session/command/check.generated.js";
export { Document } from "./session/command/format.generated.js";
export type {
    FormatOutput,
    FormatRequest,
} from "./session/command/format.generated.js";
export { Scope } from "./session/command/lint.generated.js";
export type {
    LintOutput,
    LintRequest,
} from "./session/command/lint.generated.js";
export type { Module } from "./session/module.generated.js";
export type { ParseOutput } from "./session/command/parse.generated.js";
export type {
    TraceArtifact,
    TraceCounter,
    TraceReport,
    TraceSpan,
    TraceStage,
    TraceTime,
} from "./repository/trace.generated.js";
export {
    openRepository,
    type Repository,
} from "./repository/repository.js";
export {
    Edit,
    Source,
    openSession,
    type Session,
} from "./session/session.js";
export {
    openWorkspace,
    type Workspace,
} from "./workspace/workspace.js";
export type { Change } from "./session/source/file.generated.js";
export type {
    Commit,
    TextEdit,
    TextRange,
} from "./session/source/update.generated.js";
