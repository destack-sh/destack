import type { SessionFile } from "./file.generated.js";
import type { CheckOutput } from "./command/check.generated.js";
import type { FormatOutput, FormatRequest } from "./command/format.generated.js";
import type { LintOutput, LintRequest } from "./command/lint.generated.js";
import type { Module } from "./module.generated.js";
import type { ParseOutput } from "./command/parse.generated.js";
import type { Change } from "./source/file.generated.js";
import type { Source as SourceInput } from "./source/source.generated.js";
import type { Commit, TextEdit } from "./source/update.generated.js";
import type * as update from "./source/update.generated.js";
import type { TraceReport } from "../repository/trace.generated.js";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { BuildOutput, BuildRequest } from "../artifact/output.generated.js";
import type { ArtifactRecord } from "../artifact/record.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { DirResolved } from "../dir/resolved.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { Content, ContentId } from "../source/file.generated.js";
import type { PackageId } from "../source/package.generated.js";
import type { ProfileId } from "../source/profile.generated.js";
import type { TargetId } from "../source/target.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import { openNapiSession } from "../napi/session.js";
import { hasNodeProcess } from "../runtime.js";
import { openWasmSession } from "../wasm/session.js";

/** A session source input. */
export type Source = SourceInput;

export type Edit = update.Edit;

/** Constructors for edits. */
export const Edit = {
    /** Replace or create one text file. */
    setText(path: string, text: string): Edit {
        return { kind: "setText", path, text };
    },

    /** Apply text replacements to one tracked text file. */
    editText(path: string, edits: readonly TextEdit[]): Edit {
        return { kind: "editText", path, edits };
    },

    /** Replace or create one binary file. */
    setBytes(path: string, bytes: Uint8Array | readonly number[]): Edit {
        return { kind: "setBytes", path, bytes };
    },

    /** Remove one file. */
    remove(path: string): Edit {
        return { kind: "remove", path };
    },

    /** Move one file. */
    move(from: string, to: string): Edit {
        return { kind: "move", from, to };
    },
};

/** Constructors for session source inputs. */
export const Source = {
    /** Create one filesystem source. */
    fileSystem(path: string): Source {
        return { kind: "fileSystem", path };
    },

    /** Create one in-memory source. */
    memory(root: string, edits: readonly Edit[]): Source {
        return { kind: "memory", root, edits };
    },
};

/** A live language session. */
export interface Session {
    /** Return the current session revision. */
    revision(): Revision;
    /** Return editable repository file paths. */
    files(): readonly SessionFile[];
    /** Edit files through the current session revision. */
    edit(edits: readonly Edit[]): Commit;
    /** Edit files when the current revision still matches. */
    editIfCurrent(revision: Revision, edits: readonly Edit[]): Commit;
    /** Reload tracked files from this session backing source. */
    reload(): readonly Change[];
    /** Return one module path in the current session. */
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

/** Open one language session. */
export async function openSession(source: Source): Promise<Session> {
    // native path sessions require node api filesystem access
    if (source.kind === "fileSystem") {
        return openNapiSession(source);
    }

    // node uses native bindings
    if (hasNodeProcess()) {
        return openNapiSession(source);
    }

    return openWasmSession(source);
}
