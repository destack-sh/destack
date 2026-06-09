export { ArtifactKey } from "./artifact/key.generated.js";
export type {
    ArtifactDependency,
    ArtifactDirectoryEntry,
    ArtifactPathState,
    ArtifactSourceDependency,
} from "./artifact/dependency.generated.js";
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
    Edit,
    FilePatch,
} from "./diagnostic/edit.generated.js";
export type { DirChecked } from "./dir/checked.generated.js";
export type { DirParsed } from "./dir/parsed.generated.js";
export type { DirResolved } from "./dir/resolved.generated.js";
export type { Revision } from "./repository/revision.generated.js";
export type { ComponentId } from "./source/component.generated.js";
export type {
    FileContent,
    FileContentId,
    FileId,
} from "./source/file.generated.js";
export type { ModuleId } from "./source/module.generated.js";
export type { PackageId } from "./source/package.generated.js";
export type { ProfileId } from "./source/profile.generated.js";
export type {
    LabeledSpan,
    Span,
} from "./source/span.generated.js";
export type { TargetId } from "./source/target.generated.js";
export type { SessionFile } from "./session/file.generated.js";
export type { Module } from "./session/module.generated.js";
export {
    FileEdit,
    Source,
    openSession,
    type Session,
} from "./session/session.js";
export type {
    FileChange,
    FileChangeKind,
} from "./session/source/file.generated.js";
export type {
    FileUpdate,
    FileUpdateResult,
    TextEdit,
    TextRange,
} from "./session/source/update.generated.js";
