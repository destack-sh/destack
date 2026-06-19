import type { ArtifactKey } from "../artifact/key.generated.js";
import type { BuildOutput, BuildRequest } from "../artifact/output.generated.js";
import type { ArtifactRecord } from "../artifact/record.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { DirResolved } from "../dir/resolved.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import { openNapiWorkspace } from "../napi/session.js";
import type { Revision } from "../repository/revision.generated.js";
import type { TraceReport } from "../repository/trace.generated.js";
import { hasNodeProcess } from "../runtime.js";
import type { CheckOutput } from "../session/command/check.generated.js";
import type { SessionFile } from "../session/file.generated.js";
import type { FormatOutput, FormatRequest } from "../session/command/format.generated.js";
import type { LintOutput, LintRequest } from "../session/command/lint.generated.js";
import type { Module } from "../session/module.generated.js";
import type { ParseOutput } from "../session/command/parse.generated.js";
import type { Change } from "../session/source/file.generated.js";
import type { Source, Edit } from "../session/session.js";
import type { Commit } from "../session/source/update.generated.js";
import type { Content, ContentId } from "../source/file.generated.js";
import type { PackageId } from "../source/package.generated.js";
import type { ProfileId } from "../source/profile.generated.js";
import type { TargetId } from "../source/target.generated.js";
import { openWasmWorkspace } from "../wasm/session.js";

/** A tooling workspace. */
export interface Workspace {
    /** Return this workspace root path. */
    root(): string;
    /** Return the current workspace revision. */
    revision(): Revision;
    /** Return editable repository file paths. */
    files(): readonly SessionFile[];
    /** Edit files through the current workspace revision. */
    edit(edits: readonly Edit[]): Commit;
    /** Edit files when the current workspace revision still matches. */
    editIfCurrent(revision: Revision, edits: readonly Edit[]): Commit;
    /** Reload tracked files from this workspace backing source. */
    reload(): readonly Change[];
    /** Return one module path in the workspace root. */
    module(path: string): Module;
    /** Return one named target in one package. */
    target(revision: Revision, packageValue: PackageId, name: string): TargetId;
    /** Return the semantic profile selected by one module target name. */
    profile(revision: Revision, module: Module, name: string): ProfileId;
    /** Provide root artifacts for one immutable revision. */
    provide(revision: Revision, keys: readonly ArtifactKey[]): void;
    /** Require one root artifact for one immutable revision. */
    require(revision: Revision, key: ArtifactKey): ArtifactVersion;
    /** Return one raw artifact record for one immutable revision. */
    artifactRecord(revision: Revision, key: ArtifactKey): ArtifactRecord;
    /** Return the trace report for the latest completed artifact run. */
    trace(revision: Revision, detailed: boolean): TraceReport | undefined;
    /** Build one typed language output for one immutable revision. */
    build(revision: Revision, request: BuildRequest): BuildOutput;
    /** Return one shared content payload by exact content id. */
    content(id: ContentId): Content;
    /** Return one text content payload by exact content id. */
    text(id: ContentId): string;
    /** Return one binary content payload by exact content id. */
    bytes(id: ContentId): Uint8Array;
    /** Parse one loaded module. */
    parse(revision: Revision, module: Module): ParseOutput;
    /** Return the resolved DIR artifact for one loaded module profile. */
    resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved;
    /** Check one loaded module profile. */
    check(revision: Revision, module: Module, profile: ProfileId): CheckOutput;
    /** Format one document for one immutable revision. */
    format(revision: Revision, request: FormatRequest): FormatOutput;
    /** Lint one scope for one immutable revision. */
    lint(revision: Revision, request: LintRequest): LintOutput;
    /** Return diagnostics for one immutable revision. */
    diagnostics(revision: Revision, key?: ArtifactKey): readonly Diagnostic[];
    /** Return sidecars for one artifact key in one immutable revision. */
    sidecars(revision: Revision, key: ArtifactKey): readonly ArtifactSidecar[];
}

/** Open one language workspace. */
export async function openWorkspace(source: Source): Promise<Workspace> {
    // native path workspaces require node api filesystem access
    if (source.kind === "fileSystem") {
        return openNapiWorkspace(source);
    }

    // node uses native bindings
    if (hasNodeProcess()) {
        return openNapiWorkspace(source);
    }

    return openWasmWorkspace(source);
}
