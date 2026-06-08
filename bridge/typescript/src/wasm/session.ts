import type * as Wasm from "@destack/language-wasm";
import type { SessionFile } from "../session/file.generated.js";
import type { Module } from "../session/module.generated.js";
import type { FileUpdate } from "../session/source/file.generated.js";
import type { SourceSnapshot } from "../session/source/snapshot.generated.js";
import type { SourceUpdate, SourceUpdateResult } from "../session/source/update.generated.js";
import type { Revision } from "../repository/revision.generated.js";
import type { Session } from "../session/session.js";
import {
    fromWasmFileUpdate,
    fromWasmModule,
    fromWasmSessionFile,
    fromWasmSourceUpdateResult,
    toWasmSourceSnapshot,
    toWasmSourceUpdate,
} from "./generated.js";

type WasmModule = typeof import("@destack/language-wasm");

/** Open one WASM language session from an explicit source snapshot. */
export async function openWasmSource(root: string, source: SourceSnapshot): Promise<Session> {
    const wasm = await import("@destack/language-wasm");
    await wasm.default();
    const session = wasm.Session.openSource(root, toWasmSourceSnapshot(wasm, source));

    return new WasmSession(wasm, session);
}

class WasmSession implements Session {
    public readonly backend = "wasm";

    public constructor(
        private readonly wasm: WasmModule,
        private readonly session: Wasm.Session,
    ) {}

    public revision(): Revision {
        return this.session.revision();
    }

    public files(): readonly SessionFile[] {
        return this.session.files().map(fromWasmSessionFile);
    }

    public update(update: SourceUpdate): SourceUpdateResult {
        const result = this.session.update(toWasmSourceUpdate(this.wasm, update));

        return fromWasmSourceUpdateResult(result);
    }

    public reload(): readonly FileUpdate[] {
        return this.session.reload().map(fromWasmFileUpdate);
    }

    public loadModule(path: string): Module {
        return fromWasmModule(this.session.loadModule(path));
    }
}
