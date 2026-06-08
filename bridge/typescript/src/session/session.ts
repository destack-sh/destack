import type { SessionFile } from "./file.generated.js";
import type { Module } from "./module.generated.js";
import type { FileUpdate } from "./source/file.generated.js";
import type { SourceSnapshot } from "./source/snapshot.generated.js";
import type { SourceUpdate, SourceUpdateResult } from "./source/update.generated.js";
import type { ArtifactKey } from "../artifact/key.generated.js";
import type { ArtifactSidecar } from "../artifact/sidecar.generated.js";
import type { ArtifactVersion } from "../artifact/version.generated.js";
import type { Diagnostic } from "../diagnostic/diagnostic.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import { openNapiPath, openNapiSource } from "../napi/session.js";
import { openWasmSource } from "../wasm/session.js";

/** Open input for one language session. */
export type OpenSessionInput =
    | {
          /** Native filesystem path. */
          readonly path: string;
      }
    | {
          /** Source root path used for logical file identity. */
          readonly root: string;
          /** Complete source snapshot. */
          readonly source: SourceSnapshot;
      };

/** A live language session. */
export interface Session {
    /** Return the current session revision. */
    revision(): Revision;
    /** Return editable repository file paths. */
    files(): readonly SessionFile[];
    /** Apply one source update. */
    update(update: SourceUpdate): SourceUpdateResult;
    /** Reload tracked files from this session source. */
    reload(): readonly FileUpdate[];
    /** Load one module path into the current session. */
    loadModule(path: string): Module;
    /** Provide root artifacts for one immutable revision. */
    provide(revision: Revision, keys: readonly ArtifactKey[]): void;
    /** Require one root artifact for one immutable revision. */
    require(revision: Revision, key: ArtifactKey): ArtifactVersion;
    /** Return diagnostics for one immutable revision. */
    diagnostics(revision: Revision, key?: ArtifactKey): readonly Diagnostic[];
    /** Return sidecars for one artifact key in one immutable revision. */
    sidecars(revision: Revision, key: ArtifactKey): readonly ArtifactSidecar[];
}

/** Open one language session. */
export async function openSession(input: OpenSessionInput): Promise<Session> {
    // native path sessions require node api filesystem access
    if ("path" in input) {
        return openNapiPath(input.path);
    }

    // node uses native bindings
    if (hasNodeProcess()) {
        return openNapiSource(input.root, input.source);
    }

    return openWasmSource(input.root, input.source);
}

/** Return whether this runtime exposes Node process metadata. */
function hasNodeProcess(): boolean {
    const processValue = (globalThis as { process?: { versions?: { node?: string } } }).process;

    return processValue?.versions?.node != null;
}
