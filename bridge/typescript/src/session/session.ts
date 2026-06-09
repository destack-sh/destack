import type { SessionFile } from "./file.generated.js";
import type { Module } from "./module.generated.js";
import type { FileChange } from "./source/file.generated.js";
import type { Source as SourceInput } from "./source/source.generated.js";
import type {
    FileEdit as FileEditInput,
    FileUpdate,
    FileUpdateResult,
    TextEdit,
} from "./source/update.generated.js";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { ArtifactRecord } from "../artifact/record.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { DirChecked } from "../dir/checked.generated.js";
import type { DirParsed } from "../dir/parsed.generated.js";
import type { DirResolved } from "../dir/resolved.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { ProfileId } from "../source/profile.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import { openNapiSession } from "../napi/session.js";
import { openWasmSession } from "../wasm/session.js";

/** A session source input. */
export type Source = SourceInput;

/** A file edit accepted by a session update. */
export type FileEdit = FileEditInput;

/** Constructors for file edits. */
export const FileEdit = {
    /** Replace or create one text file. */
    setText(path: string, text: string): FileEdit {
        return { kind: "setText", path, text };
    },

    /** Apply text replacements to one tracked text file. */
    editText(path: string, edits: readonly TextEdit[]): FileEdit {
        return { kind: "editText", path, edits };
    },

    /** Replace or create one binary file. */
    setBytes(path: string, bytes: Uint8Array | readonly number[]): FileEdit {
        return { kind: "setBytes", path, bytes };
    },

    /** Remove one file. */
    remove(path: string): FileEdit {
        return { kind: "remove", path };
    },

    /** Move one file. */
    move(from: string, to: string): FileEdit {
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
    memory(root: string, edits: readonly FileEdit[]): Source {
        return { kind: "memory", root, edits };
    },
};

/** A live language session. */
export interface Session {
    /** Return the current session revision. */
    revision(): Revision;
    /** Return editable repository file paths. */
    files(): readonly SessionFile[];
    /** Apply one file update. */
    update(update: FileUpdate): FileUpdateResult;
    /** Reload tracked files from this session backing source. */
    reload(): readonly FileChange[];
    /** Load one module path into the current session. */
    loadModule(path: string): Module;
    /** Provide root artifacts for one immutable revision. */
    provide(revision: Revision, keys: readonly ArtifactKey[]): void;
    /** Require one root artifact for one immutable revision. */
    require(revision: Revision, key: ArtifactKey): ArtifactVersion;
    /** Return one raw artifact record for one immutable revision. */
    artifactRecord(revision: Revision, key: ArtifactKey): ArtifactRecord;
    /** Return the parsed DIR artifact for one loaded module. */
    parse(revision: Revision, module: Module): DirParsed;
    /** Return the resolved DIR artifact for one loaded module profile. */
    resolve(revision: Revision, module: Module, profile: ProfileId): DirResolved;
    /** Return the checked DIR facade artifact for one loaded module profile. */
    check(revision: Revision, module: Module, profile: ProfileId): DirChecked;
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

/** Return whether this runtime exposes Node process metadata. */
function hasNodeProcess(): boolean {
    const processValue = (globalThis as { process?: { versions?: { node?: string } } }).process;

    return processValue?.versions?.node != null;
}
