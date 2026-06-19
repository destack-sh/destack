import { openNapiRepository } from "../napi/session.js";
import { hasNodeProcess } from "../runtime.js";
import type { Source } from "../session/session.js";
import { openWasmRepository } from "../wasm/session.js";
import type { Workspace } from "../workspace/workspace.js";

/** A durable language repository. */
export interface Repository {
    /** Return this repository root path. */
    root(): string;
    /** Open one workspace over this repository. */
    workspace(): Workspace;
}

/** Open one language repository. */
export async function openRepository(source: Source): Promise<Repository> {
    // native path repositories require node api filesystem access
    if (source.kind === "fileSystem") {
        return openNapiRepository(source);
    }

    // node uses native bindings
    if (hasNodeProcess()) {
        return openNapiRepository(source);
    }

    return openWasmRepository(source);
}
