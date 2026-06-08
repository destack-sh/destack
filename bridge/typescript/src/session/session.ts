import type { SessionFile } from "./file.generated.js";
import type { Module } from "./module.generated.js";
import type { FileUpdate } from "./source/file.generated.js";
import type { SourceSnapshot } from "./source/snapshot.generated.js";
import type { SourceUpdate, SourceUpdateResult } from "./source/update.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import { detectBackend, type LanguageBackend } from "../backend.js";
import { openNapiPath, openNapiSource } from "../napi/session.js";
import { openWasmSource } from "../wasm/session.js";

/** Open input for one language session. */
export type OpenSessionInput =
    | {
          /** Native filesystem path. */
          readonly path: string;
          /** Backend override. */
          readonly backend?: "napi";
      }
    | {
          /** Source root path used for logical file identity. */
          readonly root: string;
          /** Complete source snapshot. */
          readonly source: SourceSnapshot;
          /** Backend override. */
          readonly backend?: LanguageBackend;
      };

/** A live language session. */
export interface Session {
    /** Selected backend. */
    readonly backend: LanguageBackend;
    /** Return the current session revision. */
    revision(): Revision;
    /** Return editable repository file paths. */
    files(): readonly SessionFile[];
    /** Apply one source update. */
    update(update: SourceUpdate): SourceUpdateResult;
    /** Reload tracked files from the backend source. */
    reload(): readonly FileUpdate[];
    /** Load one module path into the current session. */
    loadModule(path: string): Module;
}

/** Open one language session. */
export async function openSession(input: OpenSessionInput): Promise<Session> {
    const backend = input.backend ?? detectBackend();

    // native path sessions require node api filesystem access
    if ("path" in input) {
        if (backend !== "napi") {
            throw new Error("openSession({ path }) requires the napi backend");
        }

        return openNapiPath(input.path);
    }

    // source sessions work in both backends
    if (backend === "napi") {
        return openNapiSource(input.root, input.source);
    }

    return openWasmSource(input.root, input.source);
}
